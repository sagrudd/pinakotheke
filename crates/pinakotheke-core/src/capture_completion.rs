// SPDX-License-Identifier: MPL-2.0
//! Verified host-worker completion of pending website image captures.

#![allow(missing_docs)]

use crate::{
    acquisition::{Acquisition, AcquisitionError, VerifiedObject},
    gallery_catalogue::GalleryCatalogueStore,
    persistent_gallery_admission::{
        GalleryImagePresentation, PersistentGalleryAdmissionError,
        PersistentGalleryAdmissionOutcome, PersistentWebsiteGalleryAdmission,
    },
    reconciliation::{
        AuthorityObservation, ReconciliationCatalogue, ReconciliationError, ReconciliationOutcome,
        ReconciliationRequest,
    },
    viewed_media::{CapturePlanError, CapturePlanService},
    website_capture_review::WebsiteCaptureReviewAdmission,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCaptureCompletion {
    pub plan_id: String,
    pub catalogue_id: String,
    pub title: String,
    pub content_type: String,
    pub content_length: u64,
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub object_version: u64,
    pub checksum_sha256: String,
    pub verified_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureCompletionOutcome {
    ThumbnailInserted,
    OriginalAttached,
    AlreadyPresent,
}

#[derive(Debug)]
pub enum CaptureCompletionError {
    UnknownPlan,
    InvalidEvidence,
    Acquisition(AcquisitionError),
    Reconciliation(ReconciliationError),
    ReconciliationConflict,
    Gallery(PersistentGalleryAdmissionError),
    Journal(CapturePlanError),
}

impl std::fmt::Display for CaptureCompletionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "capture completion rejected: {self:?}")
    }
}

impl std::error::Error for CaptureCompletionError {}

pub fn complete_verified_image(
    plans: &mut CapturePlanService,
    gallery: GalleryCatalogueStore,
    actor_id: &str,
    evidence: VerifiedCaptureCompletion,
) -> Result<CaptureCompletionOutcome, CaptureCompletionError> {
    if evidence.verified_at_epoch_seconds == 0 {
        return Err(CaptureCompletionError::InvalidEvidence);
    }
    let plan = plans
        .pending(actor_id, &evidence.plan_id)
        .ok_or(CaptureCompletionError::UnknownPlan)?;
    let object = VerifiedObject::new_versioned(
        evidence.endpoint_id,
        evidence.object_store_id,
        evidence.object_key,
        evidence.object_version,
        evidence.checksum_sha256,
    )
    .map_err(CaptureCompletionError::Acquisition)?;
    let reconciliation = ReconciliationCatalogue::default();
    let identity = WebsiteCaptureReviewAdmission::canonical_media_identity(&plan, &reconciliation);
    let mut acquisition =
        Acquisition::discovered(identity.clone()).map_err(CaptureCompletionError::Acquisition)?;
    acquisition
        .claim(format!("complete-{}", plan.plan_id))
        .and_then(|_| acquisition.start_transfer())
        .and_then(|_| acquisition.record_stored())
        .and_then(|_| acquisition.verify(object.clone()))
        .and_then(|_| acquisition.commit())
        .map_err(CaptureCompletionError::Acquisition)?;
    let request = ReconciliationRequest::new(
        identity,
        object.checksum_sha256.clone(),
        [plan.canonical_media_url.clone()],
    )
    .map_err(CaptureCompletionError::Reconciliation)?;
    let mut reconciliation = ReconciliationCatalogue::default();
    match reconciliation
        .reconcile(request, AuthorityObservation::Verified(object))
        .map_err(CaptureCompletionError::Reconciliation)?
    {
        ReconciliationOutcome::Committed { .. }
        | ReconciliationOutcome::AlreadyCommitted { .. } => {}
        _ => return Err(CaptureCompletionError::ReconciliationConflict),
    }
    let outcome = PersistentWebsiteGalleryAdmission::new(gallery)
        .admit_image(
            &acquisition,
            &plan,
            &reconciliation,
            GalleryImagePresentation {
                catalogue_id: plan.catalogue_id.clone(),
                title: evidence.title,
                content_type: evidence.content_type,
                content_length: evidence.content_length,
            },
            evidence.verified_at_epoch_seconds,
        )
        .map_err(CaptureCompletionError::Gallery)?;
    plans
        .settle(actor_id, &plan.plan_id)
        .map_err(CaptureCompletionError::Journal)?;
    Ok(match outcome {
        PersistentGalleryAdmissionOutcome::ThumbnailInserted => {
            CaptureCompletionOutcome::ThumbnailInserted
        }
        PersistentGalleryAdmissionOutcome::OriginalAttached => {
            CaptureCompletionOutcome::OriginalAttached
        }
        PersistentGalleryAdmissionOutcome::AlreadyPresent => {
            CaptureCompletionOutcome::AlreadyPresent
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewed_media::{
        AdapterKind, CAPTURE_REQUEST_SCHEMA_VERSION, CaptureKind, CapturePairing,
        CapturePlanRequest, SiteCapturePolicy,
    };

    const CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn verified_completion_updates_gallery_and_converges_after_restart() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-completion-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let journal = root.join("plans.json");
        let gallery = GalleryCatalogueStore::new(root.join("gallery.json"));
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
                allow_explicit_originals: true,
                max_candidates_per_page: 8,
            }]
        };
        let mut plans = CapturePlanService::with_journal(pairings(), sites(), &journal).unwrap();
        let plan = plans
            .plan(
                "actor",
                1,
                CapturePlanRequest {
                    schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
                    pairing_id: "pair-0".into(),
                    origin: "https://example.invalid".into(),
                    page_url: "https://example.invalid/gallery".into(),
                    adapter_kind: AdapterKind::ExperimentalGeneric,
                    adapter_version: "1.0.0".into(),
                    capture_kind: CaptureKind::ObservedThumbnail,
                    media_url: "https://example.invalid/thumb.jpg".into(),
                    presentation_url: Some("https://media.example.invalid/original.jpg".into()),
                    width: 320,
                    height: 200,
                },
            )
            .unwrap();
        let evidence = || VerifiedCaptureCompletion {
            plan_id: plan.plan_id.clone(),
            catalogue_id: "website-card-1".into(),
            title: "Synthetic artwork".into(),
            content_type: "image/jpeg".into(),
            content_length: 42,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "thumb-object".into(),
            object_version: 3,
            checksum_sha256: CHECKSUM.into(),
            verified_at_epoch_seconds: 2,
        };
        assert_eq!(
            complete_verified_image(&mut plans, gallery.clone(), "actor", evidence()).unwrap(),
            CaptureCompletionOutcome::ThumbnailInserted
        );
        assert!(plans.pending_for_actor("actor").is_empty());
        assert!(plans.is_settled("actor", &plan.plan_id));
        let stored = gallery.load_or_empty().unwrap();
        assert_eq!(stored.items().len(), 1);
        assert_eq!(stored.items()[0].catalogue_id, plan.catalogue_id);
        assert_eq!(
            complete_verified_image(&mut plans, gallery.clone(), "actor", evidence()).unwrap(),
            CaptureCompletionOutcome::AlreadyPresent
        );
        let original = plans
            .plan(
                "actor",
                3,
                CapturePlanRequest {
                    schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
                    pairing_id: "pair-0".into(),
                    origin: "https://example.invalid".into(),
                    page_url: "https://example.invalid/gallery".into(),
                    adapter_kind: AdapterKind::ExperimentalGeneric,
                    adapter_version: "1.0.0".into(),
                    capture_kind: CaptureKind::ExplicitOriginal,
                    media_url: "https://pbs.twimg.com/media/fixture?name=medium&format=jpg".into(),
                    presentation_url: Some(
                        "https://media.example.invalid/original.jpg?different=token".into(),
                    ),
                    width: 1920,
                    height: 1080,
                },
            )
            .unwrap();
        assert_eq!(original.catalogue_id, plan.catalogue_id);
        let mut original_evidence = evidence();
        original_evidence.plan_id = original.plan_id.clone();
        original_evidence.catalogue_id = "helper-cannot-change-card".into();
        original_evidence.object_key = "original-object".into();
        original_evidence.object_version = 4;
        original_evidence.verified_at_epoch_seconds = 4;
        assert_eq!(
            complete_verified_image(&mut plans, gallery.clone(), "actor", original_evidence,)
                .unwrap(),
            CaptureCompletionOutcome::OriginalAttached
        );
        let enriched = gallery.load_or_empty().unwrap();
        assert_eq!(enriched.items().len(), 1);
        assert!(enriched.items()[0].preview.is_some());
        drop(plans);
        let restarted = CapturePlanService::with_journal(pairings(), sites(), &journal).unwrap();
        assert!(restarted.pending_for_actor("actor").is_empty());
        assert!(restarted.is_settled("actor", &plan.plan_id));
        assert!(restarted.is_settled("actor", &original.plan_id));
        let _ = std::fs::remove_dir_all(root);
    }
}
