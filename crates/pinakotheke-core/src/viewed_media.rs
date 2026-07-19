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
use sha2::{Digest, Sha256};

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
    ExplicitVideo,
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
    #[serde(default)]
    pub presentation_url: Option<String>,
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
    /// Exact, short-lived HTTPS retrieval capability. Never expose this in
    /// catalogue records, diagnostics, or user-facing logs.
    #[serde(skip_serializing)]
    pub retrieval_media_url: String,
    /// Immutable write-authority selection captured when this plan was
    /// admitted. Legacy journal records may be unbound, but workers must not
    /// execute them until they are replaced by a newly admitted bound plan.
    pub destination: Option<CaptureDestinationSnapshot>,
    pub canonical_presentation_url: String,
    pub catalogue_id: String,
    pub adapter_kind: AdapterKind,
    pub adapter_version: String,
    pub capture_kind: CaptureKind,
    pub width: u32,
    pub height: u32,
    pub state: CapturePlanState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureDestinationSnapshot {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub selection_revision: u64,
}

impl CaptureDestinationSnapshot {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        is_identifier(&self.endpoint_id)
            && is_identifier(&self.object_store_id)
            && self.selection_revision > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePlanState {
    AwaitingApprovedAcquisition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CaptureActivitySummary {
    pub observed_thumbnails: usize,
    pub opened_originals: usize,
    pub opened_videos: usize,
    pub pending: usize,
    pub stored: usize,
    pub last_observed_at_epoch_seconds: Option<u64>,
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
    #[must_use]
    pub fn activity_for_actor(&self, actor_id: &str) -> CaptureActivitySummary {
        let mut summary = CaptureActivitySummary::default();
        for pending in self
            .accepted
            .iter()
            .filter(|pending| pending.actor_id == actor_id)
        {
            match pending.plan.capture_kind {
                CaptureKind::ObservedThumbnail => summary.observed_thumbnails += 1,
                CaptureKind::ExplicitOriginal => summary.opened_originals += 1,
                CaptureKind::ExplicitVideo => summary.opened_videos += 1,
            }
            if pending.settled {
                summary.stored += 1;
            } else {
                summary.pending += 1;
            }
            summary.last_observed_at_epoch_seconds = Some(
                summary
                    .last_observed_at_epoch_seconds
                    .map_or(pending.admitted_at_epoch_seconds, |current| {
                        current.max(pending.admitted_at_epoch_seconds)
                    }),
            );
        }
        summary
    }

    /// Returns the active pairing reference owned by an authenticated actor.
    #[must_use]
    pub fn active_pairing_id(&self, actor_id: &str, now: u64) -> Option<String> {
        self.pairings
            .values()
            .find(|pairing| {
                pairing.actor_id == actor_id && !pairing.revoked && pairing.expires_at > now
            })
            .map(|pairing| pairing.pairing_id.clone())
    }

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
            if pending.settled {
                continue;
            }
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
            .filter(|pending| pending.actor_id == actor_id && !pending.settled)
            .map(|pending| pending.plan.clone())
            .collect()
    }

    #[must_use]
    pub fn recoverable_pending(&self, now: u64) -> Vec<(String, CapturePlan)> {
        self.accepted
            .iter()
            .filter(|pending| {
                if pending.settled || pending.plan.destination.is_none() {
                    return false;
                }
                let actor_authorized = self.pairings.values().any(|pairing| {
                    pairing.actor_id == pending.actor_id
                        && !pairing.revoked
                        && pairing.expires_at > now
                });
                let site_authorized = self.sites.get(&pending.plan.origin).is_some_and(|site| {
                    site.capture_enabled
                        && site.site_id == pending.plan.site_id
                        && site.adapter_kind == pending.plan.adapter_kind
                        && site.adapter_version == pending.plan.adapter_version
                        && match pending.plan.capture_kind {
                            CaptureKind::ObservedThumbnail => site.allow_observed_thumbnails,
                            CaptureKind::ExplicitOriginal | CaptureKind::ExplicitVideo => {
                                site.allow_explicit_originals
                            }
                        }
                });
                actor_authorized && site_authorized
            })
            .map(|pending| (pending.actor_id.clone(), pending.plan.clone()))
            .collect()
    }

    #[must_use]
    pub fn pending(&self, actor_id: &str, plan_id: &str) -> Option<CapturePlan> {
        self.accepted
            .iter()
            .find(|pending| pending.actor_id == actor_id && pending.plan.plan_id == plan_id)
            .map(|pending| pending.plan.clone())
    }

    #[must_use]
    pub fn is_settled(&self, actor_id: &str, plan_id: &str) -> bool {
        self.accepted.iter().any(|pending| {
            pending.actor_id == actor_id && pending.plan.plan_id == plan_id && pending.settled
        })
    }

    #[must_use]
    pub fn settled_for_alias(
        &self,
        actor_id: &str,
        pairing_id: &str,
        now: u64,
        origin: &str,
        adapter_version: &str,
        canonical_alias: &str,
    ) -> Option<CapturePlan> {
        self.settled_for_alias_or_presentation(
            actor_id,
            pairing_id,
            now,
            origin,
            adapter_version,
            canonical_alias,
            None,
        )
    }

    #[must_use]
    // This public policy boundary deliberately keeps every authority dimension
    // explicit at the call site; bundling them would make omitted checks easier.
    #[allow(clippy::too_many_arguments)]
    pub fn settled_for_alias_or_presentation(
        &self,
        actor_id: &str,
        pairing_id: &str,
        now: u64,
        origin: &str,
        adapter_version: &str,
        canonical_alias: &str,
        canonical_presentation: Option<&str>,
    ) -> Option<CapturePlan> {
        let pairing = self.pairings.get(pairing_id)?;
        if pairing.actor_id != actor_id || pairing.revoked || pairing.expires_at <= now {
            return None;
        }
        self.accepted
            .iter()
            .filter(|pending| {
                pending.actor_id == actor_id
                    && pending.settled
                    && pending.plan.origin == origin
                    && pending.plan.adapter_version == adapter_version
                    && (cache_alias_identity(&pending.plan.canonical_media_url)
                        == cache_alias_identity(canonical_alias)
                        || (pending.plan.capture_kind == CaptureKind::ExplicitVideo
                            && canonical_presentation.is_some_and(|presentation| {
                                pending.plan.canonical_presentation_url == presentation
                            })))
            })
            .max_by_key(|pending| match pending.plan.capture_kind {
                CaptureKind::ObservedThumbnail => 0,
                CaptureKind::ExplicitOriginal => 1,
                CaptureKind::ExplicitVideo => 2,
            })
            .map(|pending| pending.plan.clone())
    }

    pub fn settle(&mut self, actor_id: &str, plan_id: &str) -> Result<(), CapturePlanError> {
        let Some(index) = self
            .accepted
            .iter()
            .position(|pending| pending.actor_id == actor_id && pending.plan.plan_id == plan_id)
        else {
            return Err(CapturePlanError::InvalidRequest);
        };
        let mut replacement = self.accepted.clone();
        replacement[index].settled = true;
        if let Some(journal) = &self.journal {
            journal
                .replace(&replacement)
                .map_err(|_| CapturePlanError::Persistence)?;
        }
        self.accepted = replacement;
        Ok(())
    }

    pub fn plan(
        &mut self,
        actor_id: &str,
        now: u64,
        request: CapturePlanRequest,
    ) -> Result<CapturePlan, CapturePlanError> {
        self.plan_internal(actor_id, now, request, None)
    }

    /// Admit a production capture plan bound to the exact reviewed
    /// endpoint/ObjectStore selection revision in force at admission time.
    pub fn plan_with_destination(
        &mut self,
        actor_id: &str,
        now: u64,
        request: CapturePlanRequest,
        destination: CaptureDestinationSnapshot,
    ) -> Result<CapturePlan, CapturePlanError> {
        if !destination.is_valid() {
            return Err(CapturePlanError::InvalidRequest);
        }
        self.plan_internal(actor_id, now, request, Some(destination))
    }

    fn plan_internal(
        &mut self,
        actor_id: &str,
        now: u64,
        request: CapturePlanRequest,
        destination: Option<CaptureDestinationSnapshot>,
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
            CaptureKind::ExplicitVideo if !site.allow_explicit_originals => {
                return Err(CapturePlanError::CaptureNotEligible);
            }
            _ => {}
        }
        let canonical_page_url = canonical_page_url(&request.origin, &request.page_url)
            .ok_or(CapturePlanError::InvalidRequest)?;
        let retrieval_media =
            retrieval_media_url(&request.media_url).ok_or(CapturePlanError::InvalidRequest)?;
        let canonical_media =
            canonical_media_url(&request.media_url).ok_or(CapturePlanError::InvalidRequest)?;
        let canonical_presentation_url = match request.presentation_url.as_deref() {
            Some(value) => canonical_media_url(value).ok_or(CapturePlanError::InvalidRequest)?,
            None => canonical_media.clone(),
        };
        if let Some(index) = self.accepted.iter().position(|pending| {
            pending.actor_id == actor_id
                && pending.plan.site_id == site.site_id
                && cache_alias_identity(&pending.plan.canonical_media_url)
                    == cache_alias_identity(&canonical_media)
                && pending.plan.capture_kind == request.capture_kind
                && pending.plan.destination == destination
        }) {
            // Signed delivery capabilities can rotate while the stable media
            // identity remains unchanged. Refresh only an unsettled plan so a
            // retry can use the newest browser-observed capability.
            if !self.accepted[index].settled
                && self.accepted[index].plan.retrieval_media_url != retrieval_media
            {
                let mut replacement = self.accepted.clone();
                replacement[index].plan.retrieval_media_url = retrieval_media;
                if let Some(journal) = &self.journal {
                    journal
                        .replace(&replacement)
                        .map_err(|_| CapturePlanError::Persistence)?;
                }
                self.accepted = replacement;
            }
            return Ok(self.accepted[index].plan.clone());
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
        // Preserve an existing legacy card identity when the same immutable X
        // image is promoted from an observed thumbnail to an opened original.
        // New X image cards use a media-derived identity that is independent of
        // transient page and presentation routes.
        let related_x_image = x_image_catalogue_id(&canonical_media).and_then(|_| {
            self.accepted
                .iter()
                .filter(|pending| {
                    pending.actor_id == actor_id
                        && pending.plan.site_id == site.site_id
                        && pending.plan.destination == destination
                        && pending.plan.capture_kind != CaptureKind::ExplicitVideo
                        && cache_alias_identity(&pending.plan.canonical_media_url)
                            == cache_alias_identity(&canonical_media)
                })
                .max_by_key(|pending| pending.settled)
                .map(|pending| pending.plan.catalogue_id.clone())
        });
        let catalogue_id = related_x_image.unwrap_or_else(|| {
            capture_catalogue_id_for_request(
                &site.site_id,
                &canonical_page_url,
                &canonical_presentation_url,
                &canonical_media,
                request.capture_kind,
            )
        });
        let plan = CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id,
            scheduler_job_id,
            site_id: site.site_id.clone(),
            origin: request.origin,
            canonical_page_url,
            canonical_media_url: canonical_media,
            retrieval_media_url: retrieval_media,
            destination,
            catalogue_id,
            canonical_presentation_url,
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
            settled: false,
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

pub(crate) fn cache_alias_identity(value: &str) -> &str {
    if value.starts_with("https://pbs.twimg.com/media/") {
        value.split('?').next().unwrap_or(value)
    } else {
        value
    }
}

fn validate_request(request: &CapturePlanRequest) -> Result<(), CapturePlanError> {
    if request.schema_version != CAPTURE_REQUEST_SCHEMA_VERSION
        || !is_identifier(&request.pairing_id)
        || !is_https_origin(&request.origin)
        || canonical_page_url(&request.origin, &request.page_url).is_none()
        || !is_semver(&request.adapter_version)
        || canonical_media_url(&request.media_url).is_none()
        || retrieval_media_url(&request.media_url).is_none()
        || request
            .presentation_url
            .as_deref()
            .is_some_and(|value| canonical_media_url(value).is_none())
        || request.width == 0
        || request.height == 0
        || request.width > 32_768
        || request.height > 32_768
    {
        return Err(CapturePlanError::InvalidRequest);
    }
    Ok(())
}

pub(crate) fn retrieval_media_url(value: &str) -> Option<String> {
    if value.len() > 4_096
        || !value.starts_with("https://")
        || value.contains([' ', '\n', '\r', '@'])
    {
        return None;
    }
    let exact = value.split('#').next()?;
    let host_and_path = exact.strip_prefix("https://")?;
    let host = host_and_path.split('/').next()?.split('?').next()?;
    if host.is_empty() || host.contains(':') || host.contains('*') {
        return None;
    }
    Some(exact.to_owned())
}

#[must_use]
pub fn capture_catalogue_id(site_id: &str, page_url: &str, presentation_url: &str) -> String {
    let identity = format!("{page_url}\n{presentation_url}");
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    format!("website-{site_id}-{}", &digest[..24])
}

/// Stable catalogue identity for one X image, independent of rendition query,
/// page route, gallery route, or the user-defined site rule name.
#[must_use]
pub fn x_image_catalogue_id(media_url: &str) -> Option<String> {
    let canonical = canonical_media_url(media_url)?;
    if !canonical.starts_with("https://pbs.twimg.com/media/") {
        return None;
    }
    let identity = cache_alias_identity(&canonical);
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("pinakotheke:x-image:v1\n{identity}").as_bytes())
    );
    Some(format!("website-x-media-{}", &digest[..24]))
}

/// Catalogue identity for a newly admitted request. Generic-site image cards
/// retain presentation grouping; X image cards use their immutable media path.
#[must_use]
pub fn capture_catalogue_id_for_request(
    site_id: &str,
    page_url: &str,
    presentation_url: &str,
    media_url: &str,
    capture_kind: CaptureKind,
) -> String {
    if capture_kind != CaptureKind::ExplicitVideo
        && let Some(identity) = x_image_catalogue_id(media_url)
    {
        return identity;
    }
    let mut identity = capture_catalogue_id(site_id, page_url, presentation_url);
    if capture_kind == CaptureKind::ExplicitVideo {
        let digest = format!("{:x}", Sha256::digest(media_url.as_bytes()));
        identity.push_str("-video-");
        identity.push_str(&digest[..12]);
    }
    identity
}

pub(crate) fn canonical_media_url(value: &str) -> Option<String> {
    if value.len() > 2_048
        || !value.starts_with("https://")
        || value.contains([' ', '\n', '\r', '@'])
    {
        return None;
    }
    let without_fragment = value.split('#').next()?;
    let (without_query, query) = without_fragment
        .split_once('?')
        .map_or((without_fragment, None), |(base, query)| {
            (base, Some(query))
        });
    let host_and_path = without_query.strip_prefix("https://")?;
    let host = host_and_path.split('/').next()?;
    if host.is_empty() || host.contains(':') || host.contains('*') {
        return None;
    }
    let mut canonical = without_query.to_owned();
    if host == "pbs.twimg.com" {
        let mut format = None;
        let mut name = None;
        for pair in query
            .unwrap_or_default()
            .split('&')
            .filter(|pair| !pair.is_empty())
        {
            let (key, value) = pair.split_once('=')?;
            if value.is_empty()
                || value.len() > 32
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            {
                return None;
            }
            match key {
                "format" if format.is_none() => format = Some(value),
                "name" if name.is_none() => name = Some(value),
                _ => {}
            }
        }
        if let Some(value) = format {
            canonical.push_str("?format=");
            canonical.push_str(value);
        }
        if let Some(value) = name {
            canonical.push(if format.is_some() { '&' } else { '?' });
            canonical.push_str("name=");
            canonical.push_str(value);
        }
    }
    Some(canonical)
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
            presentation_url: None,
            width: 320,
            height: 240,
        }
    }

    fn destination(revision: u64) -> CaptureDestinationSnapshot {
        CaptureDestinationSnapshot {
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            selection_revision: revision,
        }
    }

    #[test]
    fn destination_is_bound_at_admission_and_rotation_does_not_mutate_existing_plan() {
        let mut planner = service();
        let first = planner
            .plan_with_destination("actor", 1, request(), destination(3))
            .expect("bound plan");
        let rotated = planner
            .plan_with_destination("actor", 2, request(), destination(4))
            .expect("newly bound plan");

        assert_ne!(first.plan_id, rotated.plan_id);
        assert_eq!(first.destination, Some(destination(3)));
        assert_eq!(rotated.destination, Some(destination(4)));
        assert_eq!(
            planner
                .pending("actor", &first.plan_id)
                .unwrap()
                .destination,
            Some(destination(3))
        );
    }

    #[test]
    fn destination_rejects_invalid_ids_and_zero_revision() {
        for invalid in [
            CaptureDestinationSnapshot {
                endpoint_id: "endpoint/unsafe".into(),
                ..destination(1)
            },
            CaptureDestinationSnapshot {
                object_store_id: "".into(),
                ..destination(1)
            },
            destination(0),
        ] {
            assert_eq!(
                service().plan_with_destination("actor", 1, request(), invalid),
                Err(CapturePlanError::InvalidRequest)
            );
        }
    }

    #[test]
    fn host_bound_observed_thumbnail_becomes_a_scheduled_redacted_plan() {
        let plan = service().plan("actor", 1, request()).expect("plan");
        assert_eq!(
            plan.canonical_media_url,
            "https://example.invalid/media/thumbnail.webp"
        );
        assert_eq!(
            plan.retrieval_media_url,
            "https://example.invalid/media/thumbnail.webp?rotating=signature"
        );
        assert_eq!(plan.canonical_page_url, "https://example.invalid/gallery");
        assert_eq!(plan.capture_kind, CaptureKind::ObservedThumbnail);
        assert_eq!(plan.scheduler_job_id, "refresh-0");
        assert_eq!(plan.state, CapturePlanState::AwaitingApprovedAcquisition);
    }

    #[test]
    fn settled_alias_evidence_is_pairing_scoped_and_restart_safe() {
        let mut planner = service();
        let plan = planner.plan("actor", 1, request()).expect("plan");
        assert!(
            planner
                .settled_for_alias(
                    "actor",
                    "pair-0",
                    2,
                    &plan.origin,
                    &plan.adapter_version,
                    &plan.canonical_media_url,
                )
                .is_none()
        );
        planner.settle("actor", &plan.plan_id).expect("settle");
        assert_eq!(
            planner
                .settled_for_alias(
                    "actor",
                    "pair-0",
                    2,
                    &plan.origin,
                    &plan.adapter_version,
                    &plan.canonical_media_url,
                )
                .map(|matched| matched.plan_id),
            Some(plan.plan_id)
        );
        let site = planner
            .sites
            .get_mut("https://example.invalid")
            .expect("site");
        site.allow_explicit_originals = true;
        site.max_candidates_per_page = 3;
        let mut original_request = request();
        original_request.capture_kind = CaptureKind::ExplicitOriginal;
        let original = planner
            .plan("actor", 3, original_request)
            .expect("original plan");
        planner
            .settle("actor", &original.plan_id)
            .expect("settle original");
        assert_eq!(
            planner
                .settled_for_alias(
                    "actor",
                    "pair-0",
                    4,
                    &original.origin,
                    &original.adapter_version,
                    &original.canonical_media_url,
                )
                .map(|matched| matched.plan_id),
            Some(original.plan_id),
            "an explicit original is stronger stored evidence than an observed thumbnail",
        );
        assert!(
            planner
                .settled_for_alias(
                    "other",
                    "pair-0",
                    2,
                    &plan.origin,
                    &plan.adapter_version,
                    &plan.canonical_media_url,
                )
                .is_none()
        );
        let mut video_request = request();
        video_request.capture_kind = CaptureKind::ExplicitVideo;
        video_request.media_url = "https://video.example.invalid/fixture.m3u8".into();
        video_request.presentation_url = Some("https://example.invalid/status/42".into());
        let video = planner.plan("actor", 3, video_request).expect("video plan");
        planner
            .settle("actor", &video.plan_id)
            .expect("settle video");
        assert_eq!(
            planner
                .settled_for_alias_or_presentation(
                    "actor",
                    "pair-0",
                    4,
                    &video.origin,
                    &video.adapter_version,
                    "https://example.invalid/status/42",
                    Some("https://example.invalid/status/42"),
                )
                .map(|matched| matched.plan_id),
            Some(video.plan_id)
        );
    }

    #[test]
    fn rotating_video_capability_refreshes_one_unsettled_canonical_plan() {
        let mut planner = CapturePlanService::new(
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
                allow_explicit_originals: true,
                max_candidates_per_page: 2,
            }],
        );
        let mut first = request();
        first.capture_kind = CaptureKind::ExplicitVideo;
        first.media_url = "https://cdn.example.invalid/opaque?token=first#ignored".into();
        let initial = planner.plan("actor", 1, first).expect("initial plan");
        let mut rotated = request();
        rotated.capture_kind = CaptureKind::ExplicitVideo;
        rotated.media_url = "https://cdn.example.invalid/opaque?token=second".into();
        let refreshed = planner.plan("actor", 2, rotated).expect("refreshed plan");

        assert_eq!(initial.plan_id, refreshed.plan_id);
        assert_eq!(
            refreshed.canonical_media_url,
            "https://cdn.example.invalid/opaque"
        );
        assert_eq!(
            refreshed.retrieval_media_url,
            "https://cdn.example.invalid/opaque?token=second"
        );
        assert_eq!(planner.accepted.len(), 1);
    }

    #[test]
    fn x_cdn_variant_query_is_preserved_without_admitting_arbitrary_queries() {
        assert_eq!(
            cache_alias_identity("https://pbs.twimg.com/media/fixture?format=jpg&name=small"),
            cache_alias_identity("https://pbs.twimg.com/media/fixture?format=jpg&name=900x900")
        );
        assert_ne!(
            cache_alias_identity("https://pbs.twimg.com/media/fixture-a?name=small"),
            cache_alias_identity("https://pbs.twimg.com/media/fixture-b?name=small")
        );
        assert_eq!(
            canonical_media_url(
                "https://pbs.twimg.com/media/fixture?name=small&format=jpg#fragment"
            ),
            Some("https://pbs.twimg.com/media/fixture?format=jpg&name=small".into())
        );
        assert_eq!(
            canonical_media_url("https://pbs.twimg.com/media/fixture?token=secret"),
            Some("https://pbs.twimg.com/media/fixture".into())
        );
        assert_eq!(
            canonical_media_url("https://example.invalid/image.jpg?token=secret"),
            Some("https://example.invalid/image.jpg".into())
        );
    }

    #[test]
    fn x_image_catalogue_identity_ignores_page_rendition_and_site_rule_name() {
        let small = capture_catalogue_id_for_request(
            "first-rule",
            "https://x.com/home",
            "https://pbs.twimg.com/media/fixture?format=jpg&name=small",
            "https://pbs.twimg.com/media/fixture?format=jpg&name=small",
            CaptureKind::ObservedThumbnail,
        );
        let original = capture_catalogue_id_for_request(
            "renamed-rule",
            "https://x.com/fixture/status/42/photo/1",
            "https://x.com/fixture/status/42/photo/1",
            "https://pbs.twimg.com/media/fixture?format=jpg&name=orig",
            CaptureKind::ExplicitOriginal,
        );
        assert_eq!(small, original);
        assert!(small.starts_with("website-x-media-"));

        let other = x_image_catalogue_id("https://pbs.twimg.com/media/other?format=jpg&name=small")
            .unwrap();
        assert_ne!(small, other);
    }

    #[test]
    fn activity_summary_is_actor_scoped_and_distinguishes_capture_kinds() {
        let mut planner = CapturePlanService::new(
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
                allow_explicit_originals: true,
                max_candidates_per_page: 4,
            }],
        );
        let thumbnail = planner.plan("actor", 7, request()).expect("thumbnail");
        let mut original = request();
        original.capture_kind = CaptureKind::ExplicitOriginal;
        original.media_url = "https://example.invalid/media/original.webp".into();
        planner.plan("actor", 9, original).expect("original");
        let mut video = request();
        video.capture_kind = CaptureKind::ExplicitVideo;
        video.media_url = "https://example.invalid/media/video.mp4".into();
        let video = planner.plan("actor", 10, video).expect("video");
        assert!(video.catalogue_id.contains("-video-"));
        planner
            .settle("actor", &thumbnail.plan_id)
            .expect("settle thumbnail");

        assert_eq!(
            planner.activity_for_actor("actor"),
            CaptureActivitySummary {
                observed_thumbnails: 1,
                opened_originals: 1,
                opened_videos: 1,
                pending: 2,
                stored: 1,
                last_observed_at_epoch_seconds: Some(10),
            }
        );
        assert_eq!(
            planner.activity_for_actor("different-actor"),
            CaptureActivitySummary::default()
        );
    }

    #[test]
    fn linked_thumbnail_and_distinct_opened_original_share_server_catalogue_identity() {
        let mut planner = CapturePlanService::new(
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
                allow_explicit_originals: true,
                max_candidates_per_page: 4,
            }],
        );
        let mut thumbnail = request();
        thumbnail.presentation_url =
            Some("https://media.example.invalid/original.jpg?signed=redacted".into());
        let thumbnail = planner.plan("actor", 1, thumbnail).unwrap();
        let mut original = request();
        original.capture_kind = CaptureKind::ExplicitOriginal;
        original.media_url = "https://media.example.invalid/original.jpg?new=signature".into();
        original.presentation_url = Some(original.media_url.clone());
        let original = planner.plan("actor", 2, original).unwrap();
        assert_ne!(thumbnail.canonical_media_url, original.canonical_media_url);
        assert_eq!(
            thumbnail.canonical_presentation_url,
            original.canonical_presentation_url
        );
        assert_eq!(thumbnail.catalogue_id, original.catalogue_id);
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
        let accepted = first
            .plan_with_destination("actor", 1, request(), destination(5))
            .expect("accepted");
        drop(first);
        let mut restarted =
            CapturePlanService::with_journal(pairings(), sites(), &journal).expect("restart");
        assert_eq!(restarted.pending_for_actor("actor"), vec![accepted.clone()]);
        assert_eq!(restarted.recoverable_pending(2).len(), 1);
        assert!(restarted.recoverable_pending(100).is_empty());
        assert!(restarted.pending_for_actor("different-actor").is_empty());
        assert_eq!(
            restarted
                .plan_with_destination("actor", 2, request(), destination(5))
                .expect("idempotent retry"),
            accepted
        );
        assert_eq!(restarted.pending_for_actor("actor").len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn restart_keeps_legacy_unbound_plan_visible_but_not_worker_recoverable() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-unbound-restart-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let journal = root.join("capture-plans.json");
        let mut first = CapturePlanService::with_journal(
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
            &journal,
        )
        .expect("first start");
        let legacy = first.plan("actor", 1, request()).expect("legacy plan");
        assert!(legacy.destination.is_none());
        drop(first);

        let restarted = CapturePlanService::with_journal(
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
            &journal,
        )
        .expect("restart");
        assert_eq!(restarted.pending_for_actor("actor"), vec![legacy]);
        assert!(restarted.recoverable_pending(2).is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
