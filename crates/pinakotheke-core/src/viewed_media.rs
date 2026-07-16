// SPDX-License-Identifier: MPL-2.0
//! Host-authenticated admission of bounded, observed browser-media plans.
//!
//! This module handles metadata only.  It never accepts browser bytes, site
//! cookies, authorization headers, form bodies, or credentials.  A future
//! worker must revalidate and acquire an admitted plan through the approved
//! DASObjectStore boundary before it can become a committed catalogue record.

#![allow(missing_docs)]

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    capture_plan_journal::{CapturePlanJournal, CapturePlanJournalError, PendingCapturePlan},
    scheduler::{JobBudget, JobKind, RefreshOutcome, Scheduler, SourceScope},
};

pub const CAPTURE_REQUEST_SCHEMA_VERSION: &str = "x-img.capture-request.v1";
pub const CAPTURE_PLAN_SCHEMA_VERSION: &str = "x-img.capture-plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    ObservedThumbnail,
    ExplicitOriginal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Explicit,
    ExperimentalGeneric,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePlanRequest {
    pub schema_version: String,
    pub pairing_id: String,
    pub origin: String,
    pub page_url: String,
    pub adapter_kind: AdapterKind,
    pub adapter_version: String,
    pub capture_kind: CaptureKind,
    pub media_url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePlan {
    pub schema_version: &'static str,
    pub plan_id: String,
    pub scheduler_job_id: String,
    pub site_id: String,
    pub origin: String,
    pub canonical_page_url: String,
    pub canonical_media_url: String,
    pub adapter_kind: AdapterKind,
    pub adapter_version: String,
    pub capture_kind: CaptureKind,
    pub width: u32,
    pub height: u32,
    pub state: CapturePlanState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePlanState {
    AwaitingApprovedAcquisition,
}

/// A Monas-issued pairing reference bound to one actor.  The reference is an
/// opaque correlation value, not a credential: the host context is still
/// mandatory for every request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePairing {
    pub pairing_id: String,
    pub actor_id: String,
    pub expires_at: u64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteCapturePolicy {
    pub site_id: String,
    pub origin: String,
    pub capture_enabled: bool,
    pub adapter_kind: AdapterKind,
    pub adapter_version: String,
    pub allow_observed_thumbnails: bool,
    pub allow_explicit_originals: bool,
    pub max_candidates_per_page: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapturePlanError {
    InvalidRequest,
    UnknownPairing,
    PairingActorMismatch,
    PairingExpired,
    PairingRevoked,
    SiteNotEnabled,
    AdapterMismatch,
    CaptureNotEligible,
    CandidateBudgetExceeded,
    Scheduler,
    Persistence,
}

impl std::fmt::Display for CapturePlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "capture request is invalid",
            Self::UnknownPairing => "extension pairing is not recognized",
            Self::PairingActorMismatch => "extension pairing is not owned by this actor",
            Self::PairingExpired => "extension pairing has expired",
            Self::PairingRevoked => "extension pairing has been revoked",
            Self::SiteNotEnabled => "this site is not enabled for capture",
            Self::AdapterMismatch => "site adapter does not match the enabled policy",
            Self::CaptureNotEligible => "the requested media event is not eligible",
            Self::CandidateBudgetExceeded => "the site candidate budget has been reached",
            Self::Scheduler => "capture could not be admitted to the scheduler",
            Self::Persistence => "capture plan could not be persisted",
        })
    }
}

impl std::error::Error for CapturePlanError {}

/// Policy boundary for a host composition. Monas pairing issuance and worker
/// execution remain authority adapters; an optional journal preserves accepted
/// metadata across process restarts.
#[derive(Debug)]
pub struct CapturePlanService {
    pairings: BTreeMap<String, CapturePairing>,
    sites: BTreeMap<String, SiteCapturePolicy>,
    scheduler: Scheduler,
    next_plan: u64,
    journal: Option<CapturePlanJournal>,
    accepted: Vec<PendingCapturePlan>,
}

impl CapturePlanService {
    pub fn new(
        pairings: impl IntoIterator<Item = CapturePairing>,
        sites: impl IntoIterator<Item = SiteCapturePolicy>,
    ) -> Self {
        Self {
            pairings: pairings
                .into_iter()
                .map(|pairing| (pairing.pairing_id.clone(), pairing))
                .collect(),
            sites: sites
                .into_iter()
                .map(|site| (site.origin.clone(), site))
                .collect(),
            scheduler: Scheduler::default(),
            next_plan: 0,
            journal: None,
            accepted: Vec::new(),
        }
    }

    pub fn with_journal(
        pairings: impl IntoIterator<Item = CapturePairing>,
        sites: impl IntoIterator<Item = SiteCapturePolicy>,
        path: impl Into<PathBuf>,
    ) -> Result<Self, CapturePlanJournalError> {
        let journal = CapturePlanJournal::new(path);
        let accepted = journal.load()?;
        let next_plan = accepted
            .iter()
            .filter_map(|pending| pending.plan.plan_id.strip_prefix("capture-plan-"))
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .max()
            .map_or(0, |value| value.saturating_add(1));
        let mut service = Self::new(pairings, sites);
        service.journal = Some(journal);
        service.next_plan = next_plan;
        for pending in &accepted {
            let restored_job = service
                .schedule(&pending.actor_id, 1_000, pending.plan.plan_id.clone())
                .map_err(|_| CapturePlanJournalError::InvalidRecord)?;
            if restored_job != pending.plan.scheduler_job_id {
                return Err(CapturePlanJournalError::InvalidRecord);
            }
        }
        service.accepted = accepted;
        Ok(service)
    }

    #[must_use]
    pub fn pending_for_actor(&self, actor_id: &str) -> Vec<CapturePlan> {
        self.accepted
            .iter()
            .filter(|pending| pending.actor_id == actor_id)
            .map(|pending| pending.plan.clone())
            .collect()
    }

    pub fn plan(
        &mut self,
        actor_id: &str,
        now: u64,
        request: CapturePlanRequest,
    ) -> Result<CapturePlan, CapturePlanError> {
        validate_request(&request)?;
        let pairing = self
            .pairings
            .get(&request.pairing_id)
            .ok_or(CapturePlanError::UnknownPairing)?;
        if pairing.actor_id != actor_id {
            return Err(CapturePlanError::PairingActorMismatch);
        }
        if pairing.revoked {
            return Err(CapturePlanError::PairingRevoked);
        }
        if pairing.expires_at <= now {
            return Err(CapturePlanError::PairingExpired);
        }
        let site = self
            .sites
            .get(&request.origin)
            .ok_or(CapturePlanError::SiteNotEnabled)?
            .clone();
        if !site.capture_enabled {
            return Err(CapturePlanError::SiteNotEnabled);
        }
        if site.adapter_kind != request.adapter_kind
            || site.adapter_version != request.adapter_version
        {
            return Err(CapturePlanError::AdapterMismatch);
        }
        match request.capture_kind {
            CaptureKind::ObservedThumbnail if !site.allow_observed_thumbnails => {
                return Err(CapturePlanError::CaptureNotEligible);
            }
            CaptureKind::ExplicitOriginal if !site.allow_explicit_originals => {
                return Err(CapturePlanError::CaptureNotEligible);
            }
            _ => {}
        }
        let canonical_page_url = canonical_page_url(&request.origin, &request.page_url)
            .ok_or(CapturePlanError::InvalidRequest)?;
        let canonical_media_url =
            canonical_media_url(&request.media_url).ok_or(CapturePlanError::InvalidRequest)?;
        if let Some(existing) = self.accepted.iter().find(|pending| {
            pending.actor_id == actor_id
                && pending.plan.site_id == site.site_id
                && pending.plan.canonical_media_url == canonical_media_url
                && pending.plan.capture_kind == request.capture_kind
        }) {
            return Ok(existing.plan.clone());
        }
        let day = now / 86_400;
        let observed = self
            .accepted
            .iter()
            .filter(|pending| {
                pending.actor_id == actor_id
                    && pending.plan.canonical_page_url == canonical_page_url
                    && pending.admitted_at_epoch_seconds / 86_400 == day
            })
            .count() as u64;
        if observed >= site.max_candidates_per_page {
            return Err(CapturePlanError::CandidateBudgetExceeded);
        }

        let plan_id = format!("capture-plan-{}", self.next_plan);
        self.next_plan = self.next_plan.saturating_add(1);
        let scheduler_job_id =
            self.schedule(actor_id, site.max_candidates_per_page, plan_id.clone())?;
        let plan = CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id,
            scheduler_job_id,
            site_id: site.site_id.clone(),
            origin: request.origin,
            canonical_page_url,
            canonical_media_url,
            adapter_kind: request.adapter_kind,
            adapter_version: request.adapter_version,
            capture_kind: request.capture_kind,
            width: request.width,
            height: request.height,
            state: CapturePlanState::AwaitingApprovedAcquisition,
        };
        let pending = PendingCapturePlan {
            actor_id: actor_id.into(),
            admitted_at_epoch_seconds: now,
            plan: plan.clone(),
        };
        if let Some(journal) = &self.journal {
            let mut replacement = self.accepted.clone();
            replacement.push(pending.clone());
            journal
                .replace(&replacement)
                .map_err(|_| CapturePlanError::Persistence)?;
        }
        self.accepted.push(pending);
        Ok(plan)
    }

    fn schedule(
        &mut self,
        actor_id: &str,
        max_requests: u64,
        plan_id: String,
    ) -> Result<String, CapturePlanError> {
        let source = SourceScope::new(JobKind::ExtensionCapture, plan_id)
            .map_err(|_| CapturePlanError::Scheduler)?;
        let actor_scope = format!("capture-{actor_id}");
        match self.scheduler.request_refresh(
            actor_scope.clone(),
            [source.clone()],
            JobBudget {
                max_concurrent_children: 1,
                max_requests,
                max_bytes: 0,
                max_duration_seconds: 60,
            },
        ) {
            Ok(RefreshOutcome::Started { job_id }) => Ok(job_id),
            Ok(RefreshOutcome::Coalesced { .. }) => self
                .scheduler
                .admit_sources(&actor_scope, [source])
                .map_err(|_| CapturePlanError::Scheduler),
            Err(_) => Err(CapturePlanError::Scheduler),
        }
    }
}

fn validate_request(request: &CapturePlanRequest) -> Result<(), CapturePlanError> {
    if request.schema_version != CAPTURE_REQUEST_SCHEMA_VERSION
        || !is_identifier(&request.pairing_id)
        || !is_https_origin(&request.origin)
        || canonical_page_url(&request.origin, &request.page_url).is_none()
        || !is_semver(&request.adapter_version)
        || canonical_media_url(&request.media_url).is_none()
        || request.width == 0
        || request.height == 0
        || request.width > 32_768
        || request.height > 32_768
    {
        return Err(CapturePlanError::InvalidRequest);
    }
    Ok(())
}

fn canonical_media_url(value: &str) -> Option<String> {
    if value.len() > 2_048
        || !value.starts_with("https://")
        || value.contains([' ', '\n', '\r', '@'])
    {
        return None;
    }
    let without_fragment = value.split('#').next()?;
    let without_query = without_fragment.split('?').next()?;
    let host_and_path = without_query.strip_prefix("https://")?;
    let host = host_and_path.split('/').next()?;
    if host.is_empty() || host.contains(':') || host.contains('*') {
        return None;
    }
    Some(without_query.to_owned())
}

fn canonical_page_url(origin: &str, value: &str) -> Option<String> {
    let canonical = canonical_media_url(value)?;
    if canonical == origin || canonical.starts_with(&format!("{origin}/")) {
        Some(canonical)
    } else {
        None
    }
}

fn is_https_origin(value: &str) -> bool {
    canonical_media_url(value)
        .is_some_and(|canonical| canonical == value && !value[8..].contains('/'))
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_semver(value: &str) -> bool {
    let mut parts = value.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(major), Some(minor), Some(patch), None)
            if [major, minor, patch].into_iter().all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> CapturePlanService {
        CapturePlanService::new(
            [CapturePairing {
                pairing_id: "pair-0".into(),
                actor_id: "actor".into(),
                expires_at: 100,
                revoked: false,
            }],
            [SiteCapturePolicy {
                site_id: "site".into(),
                origin: "https://example.invalid".into(),
                capture_enabled: true,
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                allow_observed_thumbnails: true,
                allow_explicit_originals: false,
                max_candidates_per_page: 2,
            }],
        )
    }

    fn request() -> CapturePlanRequest {
        CapturePlanRequest {
            schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
            pairing_id: "pair-0".into(),
            origin: "https://example.invalid".into(),
            page_url: "https://example.invalid/gallery?private=redacted".into(),
            adapter_kind: AdapterKind::ExperimentalGeneric,
            adapter_version: "1.0.0".into(),
            capture_kind: CaptureKind::ObservedThumbnail,
            media_url: "https://example.invalid/media/thumbnail.webp?rotating=signature".into(),
            width: 320,
            height: 240,
        }
    }

    #[test]
    fn host_bound_observed_thumbnail_becomes_a_scheduled_redacted_plan() {
        let plan = service().plan("actor", 1, request()).expect("plan");
        assert_eq!(
            plan.canonical_media_url,
            "https://example.invalid/media/thumbnail.webp"
        );
        assert_eq!(plan.canonical_page_url, "https://example.invalid/gallery");
        assert_eq!(plan.capture_kind, CaptureKind::ObservedThumbnail);
        assert_eq!(plan.scheduler_job_id, "refresh-0");
        assert_eq!(plan.state, CapturePlanState::AwaitingApprovedAcquisition);
    }

    #[test]
    fn rejects_unpaired_or_unauthorized_or_ineligible_requests() {
        let mut planner = service();
        assert_eq!(
            planner.plan("another", 1, request()),
            Err(CapturePlanError::PairingActorMismatch)
        );
        let mut original = request();
        original.capture_kind = CaptureKind::ExplicitOriginal;
        assert_eq!(
            planner.plan("actor", 1, original),
            Err(CapturePlanError::CaptureNotEligible)
        );
        let mut unsafe_url = request();
        unsafe_url.media_url = "https://user@example.invalid/private.webp".into();
        assert_eq!(
            planner.plan("actor", 1, unsafe_url),
            Err(CapturePlanError::InvalidRequest)
        );
        let mut other_page = request();
        other_page.page_url = "https://other.invalid/gallery".into();
        assert_eq!(
            planner.plan("actor", 1, other_page),
            Err(CapturePlanError::InvalidRequest)
        );
    }

    #[test]
    fn bounds_candidates_and_adds_each_accepted_plan_to_the_common_job() {
        let mut planner = service();
        let first = planner.plan("actor", 1, request()).expect("first");
        let mut second_request = request();
        second_request.media_url = "https://example.invalid/media/second.webp".into();
        let second = planner.plan("actor", 1, second_request).expect("second");
        assert_eq!(first.scheduler_job_id, second.scheduler_job_id);
        let mut third_request = request();
        third_request.media_url = "https://example.invalid/media/third.webp".into();
        assert_eq!(
            planner.plan("actor", 1, third_request),
            Err(CapturePlanError::CandidateBudgetExceeded)
        );
    }

    #[test]
    fn journal_restarts_idempotently_and_preserves_pending_actor_scope() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-restart-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let journal = root.join("capture-plans.json");
        let pairings = || {
            [CapturePairing {
                pairing_id: "pair-0".into(),
                actor_id: "actor".into(),
                expires_at: 100,
                revoked: false,
            }]
        };
        let sites = || {
            [SiteCapturePolicy {
                site_id: "site".into(),
                origin: "https://example.invalid".into(),
                capture_enabled: true,
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                allow_observed_thumbnails: true,
                allow_explicit_originals: false,
                max_candidates_per_page: 2,
            }]
        };
        let mut first =
            CapturePlanService::with_journal(pairings(), sites(), &journal).expect("first start");
        let accepted = first.plan("actor", 1, request()).expect("accepted");
        drop(first);
        let mut restarted =
            CapturePlanService::with_journal(pairings(), sites(), &journal).expect("restart");
        assert_eq!(restarted.pending_for_actor("actor"), vec![accepted.clone()]);
        assert!(restarted.pending_for_actor("different-actor").is_empty());
        assert_eq!(
            restarted
                .plan("actor", 2, request())
                .expect("idempotent retry"),
            accepted
        );
        assert_eq!(restarted.pending_for_actor("actor").len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
}
