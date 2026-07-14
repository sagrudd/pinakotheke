// SPDX-License-Identifier: MPL-2.0
//! One-click, storage-free account-refresh orchestration.
//!
//! It schedules metadata-only X and Instagram child jobs. It never calls a
//! connector, reads credentials, downloads media, or admits review records.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use x_img_model::{InstanceConfig, RefreshBudget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountPlatform {
    X,
    Instagram,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshAccount {
    pub platform: AccountPlatform,
    pub account_id: String,
    pub budget: RefreshBudget,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRefreshState {
    Pending,
    Running,
    Completed,
    Failed,
    PolicyBlocked,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshJobState {
    Active,
    Complete,
    Partial,
    Failed,
    PolicyBlocked,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountFinish {
    Completed,
    Failed,
    PolicyBlocked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRefreshProgress {
    pub state: AccountRefreshState,
    pub attempts: u32,
    pub pages: u64,
    pub items: u64,
    pub requests: u64,
    pub bytes: u64,
    pub new_items: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressDelta {
    pub pages: u64,
    pub items: u64,
    pub requests: u64,
    pub bytes: u64,
    pub new_items: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneClickRefreshJob {
    pub job_id: String,
    pub actor_scope: String,
    pub state: RefreshJobState,
    pub children: BTreeMap<String, AccountRefreshProgress>,
    accounts: BTreeMap<String, RefreshAccount>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshSummary {
    pub completed: u32,
    pub failed: u32,
    pub policy_blocked: u32,
    pub cancelled: u32,
    pub new_items: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestOutcome {
    Started { job_id: String },
    Coalesced { job_id: String },
}
#[derive(Debug, Default)]
pub struct AccountRefreshController {
    active: BTreeMap<String, OneClickRefreshJob>,
    next_job: u64,
}

impl RefreshAccount {
    /// Produces refreshable social accounts from a strictly validated config.
    #[must_use]
    pub fn enabled_from_config(config: &InstanceConfig) -> Vec<Self> {
        let mut accounts = Vec::new();
        accounts.extend(
            config
                .x_accounts
                .iter()
                .filter(|account| account.enabled)
                .map(|account| Self {
                    platform: AccountPlatform::X,
                    account_id: account.account_id.clone(),
                    budget: account.refresh_budget.clone(),
                }),
        );
        accounts.extend(
            config
                .instagram_accounts
                .iter()
                .filter(|account| account.enabled)
                .map(|account| Self {
                    platform: AccountPlatform::Instagram,
                    account_id: account.account_id.clone(),
                    budget: account.refresh_budget.clone(),
                }),
        );
        accounts
    }
}

impl AccountRefreshController {
    /// Schedules every enabled X and Instagram account or coalesces a repeat click.
    pub fn request(
        &mut self,
        actor_scope: impl Into<String>,
        accounts: Vec<RefreshAccount>,
    ) -> Result<RequestOutcome, RefreshError> {
        let actor_scope = actor_scope.into();
        if !is_identifier(&actor_scope) {
            return Err(RefreshError::InvalidMetadata("actor_scope"));
        }
        if let Some(job) = self.active.get(&actor_scope)
            && job.state == RefreshJobState::Active
        {
            return Ok(RequestOutcome::Coalesced {
                job_id: job.job_id.clone(),
            });
        }
        let mut children = BTreeMap::new();
        let mut account_map = BTreeMap::new();
        for account in accounts {
            validate_account(&account)?;
            if children
                .insert(
                    account.account_id.clone(),
                    AccountRefreshProgress {
                        state: AccountRefreshState::Pending,
                        attempts: 0,
                        pages: 0,
                        items: 0,
                        requests: 0,
                        bytes: 0,
                        new_items: 0,
                    },
                )
                .is_some()
            {
                return Err(RefreshError::DuplicateAccount);
            }
            account_map.insert(account.account_id.clone(), account);
        }
        if children.is_empty() {
            return Err(RefreshError::NoEnabledAccounts);
        }
        let job_id = format!("account-refresh-{}", self.next_job);
        self.next_job += 1;
        self.active.insert(
            actor_scope.clone(),
            OneClickRefreshJob {
                job_id: job_id.clone(),
                actor_scope,
                state: RefreshJobState::Active,
                children,
                accounts: account_map,
            },
        );
        Ok(RequestOutcome::Started { job_id })
    }
    pub fn claim(&mut self, actor: &str, account_id: &str) -> Result<(), RefreshError> {
        let job = self.job_mut(actor)?;
        ensure_active(job)?;
        let child = job
            .children
            .get_mut(account_id)
            .ok_or(RefreshError::UnknownAccount)?;
        if child.state != AccountRefreshState::Pending {
            return Err(RefreshError::AlreadyRunningOrTerminal);
        }
        child.state = AccountRefreshState::Running;
        child.attempts = child.attempts.saturating_add(1);
        Ok(())
    }
    /// Records bounded, word-visible progress for one claimed account.
    pub fn progress(
        &mut self,
        actor: &str,
        account_id: &str,
        delta: ProgressDelta,
    ) -> Result<(), RefreshError> {
        let job = self.job_mut(actor)?;
        ensure_active(job)?;
        let budget = job
            .accounts
            .get(account_id)
            .ok_or(RefreshError::UnknownAccount)?
            .budget
            .clone();
        let child = job
            .children
            .get_mut(account_id)
            .ok_or(RefreshError::UnknownAccount)?;
        if child.state != AccountRefreshState::Running {
            return Err(RefreshError::NotRunning);
        }
        if child.pages.saturating_add(delta.pages) > budget.max_pages
            || child.items.saturating_add(delta.items) > budget.max_items
            || child.requests.saturating_add(delta.requests) > budget.max_requests
            || child.bytes.saturating_add(delta.bytes) > budget.max_bytes
        {
            return Err(RefreshError::BudgetExceeded);
        }
        child.pages += delta.pages;
        child.items += delta.items;
        child.requests += delta.requests;
        child.bytes += delta.bytes;
        child.new_items += delta.new_items;
        Ok(())
    }
    pub fn finish(
        &mut self,
        actor: &str,
        account_id: &str,
        finish: AccountFinish,
    ) -> Result<(), RefreshError> {
        let job = self.job_mut(actor)?;
        ensure_active(job)?;
        let child = job
            .children
            .get_mut(account_id)
            .ok_or(RefreshError::UnknownAccount)?;
        if child.state != AccountRefreshState::Running {
            return Err(RefreshError::NotRunning);
        }
        child.state = match finish {
            AccountFinish::Completed => AccountRefreshState::Completed,
            AccountFinish::Failed => AccountRefreshState::Failed,
            AccountFinish::PolicyBlocked => AccountRefreshState::PolicyBlocked,
        };
        recompute(job);
        Ok(())
    }
    /// Retries an explicit failure as a fresh pending child; policy blocks are not retried.
    pub fn retry(&mut self, actor: &str, account_id: &str) -> Result<(), RefreshError> {
        let job = self.job_mut(actor)?;
        let child = job
            .children
            .get_mut(account_id)
            .ok_or(RefreshError::UnknownAccount)?;
        if child.state != AccountRefreshState::Failed {
            return Err(RefreshError::NotRetryable);
        }
        child.state = AccountRefreshState::Pending;
        job.state = RefreshJobState::Active;
        Ok(())
    }
    pub fn cancel(&mut self, actor: &str) -> Result<(), RefreshError> {
        let job = self.job_mut(actor)?;
        ensure_active(job)?;
        for child in job.children.values_mut() {
            if matches!(
                child.state,
                AccountRefreshState::Pending | AccountRefreshState::Running
            ) {
                child.state = AccountRefreshState::Cancelled;
            }
        }
        job.state = RefreshJobState::Cancelled;
        Ok(())
    }
    #[must_use]
    pub fn job(&self, actor: &str) -> Option<&OneClickRefreshJob> {
        self.active.get(actor)
    }
    pub fn summary(&self, actor: &str) -> Result<RefreshSummary, RefreshError> {
        let job = self.job(actor).ok_or(RefreshError::NoActiveJob)?;
        let mut summary = RefreshSummary {
            completed: 0,
            failed: 0,
            policy_blocked: 0,
            cancelled: 0,
            new_items: 0,
        };
        for child in job.children.values() {
            summary.new_items += child.new_items;
            match child.state {
                AccountRefreshState::Completed => summary.completed += 1,
                AccountRefreshState::Failed => summary.failed += 1,
                AccountRefreshState::PolicyBlocked => summary.policy_blocked += 1,
                AccountRefreshState::Cancelled => summary.cancelled += 1,
                _ => {}
            }
        }
        Ok(summary)
    }
    fn job_mut(&mut self, actor: &str) -> Result<&mut OneClickRefreshJob, RefreshError> {
        self.active.get_mut(actor).ok_or(RefreshError::NoActiveJob)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshError {
    InvalidMetadata(&'static str),
    NoEnabledAccounts,
    DuplicateAccount,
    NoActiveJob,
    InactiveJob,
    UnknownAccount,
    AlreadyRunningOrTerminal,
    NotRunning,
    BudgetExceeded,
    NotRetryable,
}
impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "account refresh rejected: {self:?}")
    }
}
impl std::error::Error for RefreshError {}
fn ensure_active(job: &OneClickRefreshJob) -> Result<(), RefreshError> {
    if job.state == RefreshJobState::Active {
        Ok(())
    } else {
        Err(RefreshError::InactiveJob)
    }
}
fn validate_account(account: &RefreshAccount) -> Result<(), RefreshError> {
    if !is_identifier(&account.account_id)
        || account.budget.max_pages == 0
        || account.budget.max_items == 0
        || account.budget.max_requests == 0
        || account.budget.max_bytes == 0
    {
        Err(RefreshError::InvalidMetadata("account_or_budget"))
    } else {
        Ok(())
    }
}
fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | ':' | '-')
        })
}
fn recompute(job: &mut OneClickRefreshJob) {
    if job.children.values().any(|child| {
        matches!(
            child.state,
            AccountRefreshState::Pending | AccountRefreshState::Running
        )
    }) {
        return;
    }
    let summary = job
        .children
        .values()
        .fold((0, 0, 0), |(ok, fail, blocked), child| match child.state {
            AccountRefreshState::Completed => (ok + 1, fail, blocked),
            AccountRefreshState::Failed => (ok, fail + 1, blocked),
            AccountRefreshState::PolicyBlocked => (ok, fail, blocked + 1),
            _ => (ok, fail, blocked),
        });
    job.state = if summary.0 > 0 && (summary.1 > 0 || summary.2 > 0) {
        RefreshJobState::Partial
    } else if summary.0 > 0 {
        RefreshJobState::Complete
    } else if summary.1 > 0 {
        RefreshJobState::Failed
    } else {
        RefreshJobState::PolicyBlocked
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use x_img_model::InstanceConfig;
    fn accounts() -> Vec<RefreshAccount> {
        let config: InstanceConfig =
            serde_json::from_slice(include_bytes!("../../../examples/config/instance.v1.json"))
                .expect("config");
        RefreshAccount::enabled_from_config(&config)
    }
    #[test]
    fn one_click_coalesces_enabled_accounts_and_reports_partial_summary() {
        let mut controller = AccountRefreshController::default();
        let first = controller.request("actor", accounts()).expect("start");
        let repeated = controller.request("actor", accounts()).expect("coalesce");
        assert!(
            matches!((first,repeated),(RequestOutcome::Started{job_id:a},RequestOutcome::Coalesced{job_id:b}) if a==b)
        );
        let ids: Vec<_> = controller
            .job("actor")
            .expect("job")
            .children
            .keys()
            .cloned()
            .collect();
        assert_eq!(ids.len(), 2);
        controller.claim("actor", &ids[0]).expect("claim");
        assert_eq!(
            controller.claim("actor", &ids[0]),
            Err(RefreshError::AlreadyRunningOrTerminal)
        );
        controller
            .progress(
                "actor",
                &ids[0],
                ProgressDelta {
                    pages: 1,
                    items: 2,
                    requests: 1,
                    bytes: 20,
                    new_items: 2,
                },
            )
            .expect("progress");
        controller
            .finish("actor", &ids[0], AccountFinish::Completed)
            .expect("finish");
        controller.claim("actor", &ids[1]).expect("claim");
        controller
            .finish("actor", &ids[1], AccountFinish::Failed)
            .expect("fail");
        assert_eq!(
            controller.job("actor").expect("job").state,
            RefreshJobState::Partial
        );
        assert_eq!(
            controller.summary("actor").expect("summary"),
            RefreshSummary {
                completed: 1,
                failed: 1,
                policy_blocked: 0,
                cancelled: 0,
                new_items: 2
            }
        );
    }
    #[test]
    fn budgets_cancellation_and_retry_are_explicit() {
        let mut controller = AccountRefreshController::default();
        controller.request("actor", accounts()).expect("start");
        let id = controller
            .job("actor")
            .expect("job")
            .children
            .keys()
            .next()
            .expect("id")
            .clone();
        controller.claim("actor", &id).expect("claim");
        assert_eq!(
            controller.progress(
                "actor",
                &id,
                ProgressDelta {
                    pages: u64::MAX,
                    items: 0,
                    requests: 0,
                    bytes: 0,
                    new_items: 0
                }
            ),
            Err(RefreshError::BudgetExceeded)
        );
        controller
            .finish("actor", &id, AccountFinish::Failed)
            .expect("fail");
        controller.retry("actor", &id).expect("retry");
        controller.claim("actor", &id).expect("reclaim");
        controller.cancel("actor").expect("cancel");
        assert_eq!(
            controller.job("actor").expect("job").state,
            RefreshJobState::Cancelled
        );
    }
}
