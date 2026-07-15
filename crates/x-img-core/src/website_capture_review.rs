// SPDX-License-Identifier: MPL-2.0
//! Verified website-capture provenance admitted to the shared review queue.
//!
//! Capture plans are metadata only. This boundary records their provenance and
//! admits a shared `New` review item only after a verified ObjectStore object
//! has been committed through the normal acquisition lifecycle.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::{
    acquisition::{Acquisition, AcquisitionState},
    reconciliation::ReconciliationCatalogue,
    review_admission::{ReviewAdmissionError, ReviewQueue},
    viewed_media::{AdapterKind, CapturePlan},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebsiteCaptureProvenance {
    pub capture_plan_id: String,
    pub site_id: String,
    pub origin: String,
    pub canonical_page_url: String,
    pub canonical_media_url: String,
    pub adapter_kind: AdapterKind,
    pub adapter_version: String,
    pub discovery_time_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebsiteCaptureReviewError {
    NotCommitted,
    IdentityMismatch,
    Review(ReviewAdmissionError),
}

impl std::fmt::Display for WebsiteCaptureReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotCommitted => {
                formatter.write_str("capture object is not verified and committed")
            }
            Self::IdentityMismatch => {
                formatter.write_str("capture identity differs from its verified acquisition")
            }
            Self::Review(error) => write!(formatter, "review admission failed: {error}"),
        }
    }
}

impl std::error::Error for WebsiteCaptureReviewError {}

/// Metadata-only capture provenance keyed by canonical media identity.
#[derive(Debug, Default)]
pub struct WebsiteCaptureReviewAdmission {
    provenance: BTreeMap<String, WebsiteCaptureProvenance>,
}

impl WebsiteCaptureReviewAdmission {
    /// Returns the opaque stable identity a future capture worker must use.
    ///
    /// A known connector alias wins, allowing one browser observation to reuse
    /// a previously committed account-connector record. Otherwise a domain-
    /// separated SHA-256 of the redacted canonical media URL is used; no source
    /// URL is retained in the identity itself.
    #[must_use]
    pub fn canonical_media_identity(
        plan: &CapturePlan,
        catalogue: &ReconciliationCatalogue,
    ) -> String {
        if let Some(identity) = catalogue.canonical_identity_for_alias(&plan.canonical_media_url) {
            return identity.to_owned();
        }
        let digest = Sha256::digest(plan.canonical_media_url.as_bytes());
        format!("website:{digest:x}")
    }

    /// Adds website provenance and admits the existing verified commit to the
    /// common `New` review queue. Replays retain the first provenance record.
    pub fn admit(
        &mut self,
        queue: &mut ReviewQueue,
        acquisition: &Acquisition,
        plan: &CapturePlan,
        catalogue: &ReconciliationCatalogue,
        discovery_time_unix_seconds: u64,
    ) -> Result<&WebsiteCaptureProvenance, WebsiteCaptureReviewError> {
        if acquisition.state() != AcquisitionState::Committed {
            return Err(WebsiteCaptureReviewError::NotCommitted);
        }
        let identity = Self::canonical_media_identity(plan, catalogue);
        if acquisition.media_identity_id() != identity {
            return Err(WebsiteCaptureReviewError::IdentityMismatch);
        }
        queue
            .admit_new(
                acquisition,
                format!("website:{}", plan.site_id),
                discovery_time_unix_seconds,
            )
            .map_err(WebsiteCaptureReviewError::Review)?;
        self.provenance
            .entry(identity.clone())
            .or_insert(WebsiteCaptureProvenance {
                capture_plan_id: plan.plan_id.clone(),
                site_id: plan.site_id.clone(),
                origin: plan.origin.clone(),
                canonical_page_url: plan.canonical_page_url.clone(),
                canonical_media_url: plan.canonical_media_url.clone(),
                adapter_kind: plan.adapter_kind,
                adapter_version: plan.adapter_version.clone(),
                discovery_time_unix_seconds,
            });
        Ok(self.provenance.get(&identity).expect("inserted"))
    }

    #[must_use]
    pub fn provenance(&self, canonical_media_identity: &str) -> Option<&WebsiteCaptureProvenance> {
        self.provenance.get(canonical_media_identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquisition::VerifiedObject,
        reconciliation::{AuthorityObservation, ReconciliationRequest},
        viewed_media::{CAPTURE_PLAN_SCHEMA_VERSION, CaptureKind, CapturePlanState},
    };

    const CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn plan() -> CapturePlan {
        CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id: "capture-plan-0".into(),
            scheduler_job_id: "refresh-0".into(),
            site_id: "site".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://example.invalid/media/thumbnail.webp".into(),
            adapter_kind: AdapterKind::ExperimentalGeneric,
            adapter_version: "1.0.0".into(),
            capture_kind: CaptureKind::ObservedThumbnail,
            state: CapturePlanState::AwaitingApprovedAcquisition,
        }
    }

    fn committed(identity: &str) -> Acquisition {
        let mut acquisition = Acquisition::discovered(identity).expect("identity");
        acquisition.claim("worker").expect("claim");
        acquisition.start_transfer().expect("transfer");
        acquisition.record_stored().expect("stored");
        acquisition
            .verify(VerifiedObject::new("endpoint", "store", "object", CHECKSUM).expect("object"))
            .expect("verified");
        acquisition.commit().expect("commit");
        acquisition
    }

    #[test]
    fn admits_provenance_only_after_a_verified_shared_queue_commit() {
        let plan = plan();
        let catalogue = ReconciliationCatalogue::default();
        let identity = WebsiteCaptureReviewAdmission::canonical_media_identity(&plan, &catalogue);
        let mut queue = ReviewQueue::default();
        let mut admission = WebsiteCaptureReviewAdmission::default();
        let provenance = admission
            .admit(&mut queue, &committed(&identity), &plan, &catalogue, 42)
            .expect("review admission");
        assert_eq!(
            provenance.canonical_page_url,
            "https://example.invalid/gallery"
        );
        assert_eq!(provenance.adapter_version, "1.0.0");
        assert_eq!(queue.len(), 1);
        assert!(queue.get(&identity).is_some());
    }

    #[test]
    fn reuses_a_connector_identity_when_its_committed_alias_matches() {
        let plan = plan();
        let object = VerifiedObject::new("endpoint", "store", "object", CHECKSUM).expect("object");
        let mut catalogue = ReconciliationCatalogue::default();
        catalogue
            .reconcile(
                ReconciliationRequest::new(
                    "x:account:item:media",
                    CHECKSUM,
                    [plan.canonical_media_url.clone()],
                )
                .expect("request"),
                AuthorityObservation::Verified(object),
            )
            .expect("catalogue commit");
        let identity = WebsiteCaptureReviewAdmission::canonical_media_identity(&plan, &catalogue);
        assert_eq!(identity, "x:account:item:media");
        let mut queue = ReviewQueue::default();
        let mut admission = WebsiteCaptureReviewAdmission::default();
        admission
            .admit(&mut queue, &committed(&identity), &plan, &catalogue, 42)
            .expect("deduplicated review admission");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn rejects_uncommitted_or_differently_identified_acquisitions() {
        let plan = plan();
        let catalogue = ReconciliationCatalogue::default();
        let identity = WebsiteCaptureReviewAdmission::canonical_media_identity(&plan, &catalogue);
        let uncommitted = Acquisition::discovered(identity.clone()).expect("identity");
        let mut queue = ReviewQueue::default();
        let mut admission = WebsiteCaptureReviewAdmission::default();
        assert_eq!(
            admission.admit(&mut queue, &uncommitted, &plan, &catalogue, 42),
            Err(WebsiteCaptureReviewError::NotCommitted)
        );
        assert_eq!(
            admission.admit(
                &mut queue,
                &committed("website:other"),
                &plan,
                &catalogue,
                42
            ),
            Err(WebsiteCaptureReviewError::IdentityMismatch)
        );
    }
}
