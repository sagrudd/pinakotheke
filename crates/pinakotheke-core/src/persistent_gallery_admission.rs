// SPDX-License-Identifier: MPL-2.0
//! Verified website-capture admission into the persistent gallery catalogue.

#![allow(missing_docs)]

use crate::{
    acquisition::Acquisition,
    gallery_catalogue::{
        GalleryCatalogueStore, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
        GalleryRepresentation, GalleryRepresentationKind, GalleryReviewState, GallerySourceKind,
        GalleryStoreError,
    },
    reconciliation::ReconciliationCatalogue,
    review_admission::ReviewQueue,
    viewed_media::{CaptureKind, CapturePlan},
    website_capture_review::{WebsiteCaptureReviewAdmission, WebsiteCaptureReviewError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryImagePresentation {
    pub catalogue_id: String,
    pub title: String,
    pub content_type: String,
    pub content_length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentGalleryAdmissionOutcome {
    ThumbnailInserted,
    OriginalAttached,
    AlreadyPresent,
}

#[derive(Debug)]
pub enum PersistentGalleryAdmissionError {
    InvalidPresentation,
    Capture(WebsiteCaptureReviewError),
    Store(GalleryStoreError),
    OriginalRequiresThumbnail,
    DestinationMismatch,
    ConflictingReplay,
}

impl std::fmt::Display for PersistentGalleryAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "persistent gallery admission rejected: {self:?}")
    }
}

impl std::error::Error for PersistentGalleryAdmissionError {}

pub struct PersistentWebsiteGalleryAdmission {
    store: GalleryCatalogueStore,
    review_queue: ReviewQueue,
    website_review: WebsiteCaptureReviewAdmission,
}

impl PersistentWebsiteGalleryAdmission {
    #[must_use]
    pub fn new(store: GalleryCatalogueStore) -> Self {
        Self {
            store,
            review_queue: ReviewQueue::default(),
            website_review: WebsiteCaptureReviewAdmission::default(),
        }
    }

    pub fn admit_image(
        &mut self,
        acquisition: &Acquisition,
        plan: &CapturePlan,
        reconciliation: &ReconciliationCatalogue,
        presentation: GalleryImagePresentation,
        discovered_at_epoch_seconds: u64,
    ) -> Result<PersistentGalleryAdmissionOutcome, PersistentGalleryAdmissionError> {
        validate_presentation(&presentation)?;
        self.website_review
            .admit(
                &mut self.review_queue,
                acquisition,
                plan,
                reconciliation,
                discovered_at_epoch_seconds,
            )
            .map_err(PersistentGalleryAdmissionError::Capture)?;
        let object = acquisition
            .verified_object()
            .expect("website admission accepted only committed verified evidence");
        let representation = GalleryRepresentation {
            kind: match plan.capture_kind {
                CaptureKind::ObservedThumbnail => GalleryRepresentationKind::Thumbnail,
                CaptureKind::ExplicitOriginal => GalleryRepresentationKind::OriginalImage,
            },
            availability: GalleryObjectAvailability::Ready,
            endpoint_id: object.endpoint_id.clone(),
            object_store_id: object.object_store_id.clone(),
            object_key: object.object_reference_id.clone(),
            object_version: object.object_version,
            checksum: format!("sha256:{}", object.checksum_sha256),
            content_type: presentation.content_type.clone(),
            content_length: presentation.content_length,
            delivery_path: Some(format!(
                "/products/pinakotheke/api/gallery/v1/objects/{}/{}",
                presentation.catalogue_id,
                match plan.capture_kind {
                    CaptureKind::ObservedThumbnail => "thumbnail",
                    CaptureKind::ExplicitOriginal => "original",
                }
            )),
        };

        let catalogue = self
            .store
            .load_or_empty()
            .map_err(PersistentGalleryAdmissionError::Store)?;
        let mut items = catalogue.items().to_vec();
        let existing = items
            .iter_mut()
            .find(|item| item.catalogue_id == presentation.catalogue_id);
        let outcome = match (plan.capture_kind, existing) {
            (CaptureKind::ObservedThumbnail, None) => {
                items.push(GalleryItem {
                    catalogue_id: presentation.catalogue_id,
                    title: presentation.title,
                    source_label: format!("Website / {}", plan.site_id),
                    source_kind: GallerySourceKind::Website,
                    media_kind: GalleryMediaKind::Image,
                    review_state: GalleryReviewState::New,
                    discovered_at_epoch_seconds,
                    width: plan.width,
                    height: plan.height,
                    thumbnail: representation,
                    preview: None,
                });
                PersistentGalleryAdmissionOutcome::ThumbnailInserted
            }
            (CaptureKind::ObservedThumbnail, Some(item)) => {
                if item.thumbnail == representation {
                    return Ok(PersistentGalleryAdmissionOutcome::AlreadyPresent);
                }
                return Err(PersistentGalleryAdmissionError::ConflictingReplay);
            }
            (CaptureKind::ExplicitOriginal, None) => {
                return Err(PersistentGalleryAdmissionError::OriginalRequiresThumbnail);
            }
            (CaptureKind::ExplicitOriginal, Some(item)) => {
                if item.thumbnail.endpoint_id != representation.endpoint_id
                    || item.thumbnail.object_store_id != representation.object_store_id
                {
                    return Err(PersistentGalleryAdmissionError::DestinationMismatch);
                }
                if item.preview.as_ref() == Some(&representation) {
                    return Ok(PersistentGalleryAdmissionOutcome::AlreadyPresent);
                }
                if item.preview.is_some() {
                    return Err(PersistentGalleryAdmissionError::ConflictingReplay);
                }
                item.preview = Some(representation);
                item.width = plan.width;
                item.height = plan.height;
                PersistentGalleryAdmissionOutcome::OriginalAttached
            }
        };
        self.store
            .replace(items)
            .map_err(PersistentGalleryAdmissionError::Store)?;
        Ok(outcome)
    }
}

fn validate_presentation(
    presentation: &GalleryImagePresentation,
) -> Result<(), PersistentGalleryAdmissionError> {
    let safe_id = !presentation.catalogue_id.is_empty()
        && presentation.catalogue_id.len() <= 128
        && presentation.catalogue_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        });
    if !safe_id
        || presentation.title.is_empty()
        || presentation.title.len() > 256
        || presentation.title.chars().any(char::is_control)
        || !presentation.content_type.starts_with("image/")
        || presentation.content_type.len() > 128
        || presentation.content_length == 0
    {
        return Err(PersistentGalleryAdmissionError::InvalidPresentation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquisition::VerifiedObject,
        gallery_catalogue::{GalleryCatalogueStore, GalleryObjectAvailability},
        viewed_media::{AdapterKind, CAPTURE_PLAN_SCHEMA_VERSION, CapturePlanState},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    const CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn temporary_store() -> (PathBuf, GalleryCatalogueStore) {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-persistent-admission-{}-{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let store = GalleryCatalogueStore::new(root.join("gallery.json"));
        (root, store)
    }

    fn plan(kind: CaptureKind, media: &str, width: u32, height: u32) -> CapturePlan {
        CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id: format!("plan-{media}"),
            scheduler_job_id: format!("job-{media}"),
            site_id: "example-site".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: format!("https://example.invalid/{media}.jpg"),
            adapter_kind: AdapterKind::ExperimentalGeneric,
            adapter_version: "1.0.0".into(),
            capture_kind: kind,
            width,
            height,
            state: CapturePlanState::AwaitingApprovedAcquisition,
        }
    }

    fn committed(
        plan: &CapturePlan,
        reconciliation: &ReconciliationCatalogue,
        object: &str,
    ) -> Acquisition {
        committed_at(plan, reconciliation, "endpoint", "store", object)
    }

    fn committed_at(
        plan: &CapturePlan,
        reconciliation: &ReconciliationCatalogue,
        endpoint: &str,
        store: &str,
        object: &str,
    ) -> Acquisition {
        let identity =
            WebsiteCaptureReviewAdmission::canonical_media_identity(plan, reconciliation);
        let mut acquisition = Acquisition::discovered(identity).unwrap();
        acquisition.claim("worker").unwrap();
        acquisition.start_transfer().unwrap();
        acquisition.record_stored().unwrap();
        acquisition
            .verify(VerifiedObject::new_versioned(endpoint, store, object, 7, CHECKSUM).unwrap())
            .unwrap();
        acquisition.commit().unwrap();
        acquisition
    }

    fn presentation() -> GalleryImagePresentation {
        GalleryImagePresentation {
            catalogue_id: "gallery-image-1".into(),
            title: "Redistributable test image".into(),
            content_type: "image/jpeg".into(),
            content_length: 12,
        }
    }

    #[test]
    fn committed_thumbnail_and_explicit_original_persist_as_one_restart_safe_card() {
        let (root, store) = temporary_store();
        let reconciliation = ReconciliationCatalogue::default();
        let thumbnail = plan(CaptureKind::ObservedThumbnail, "thumbnail", 320, 200);
        let original = plan(CaptureKind::ExplicitOriginal, "original", 1920, 1080);
        let mut admission = PersistentWebsiteGalleryAdmission::new(store.clone());
        assert_eq!(
            admission
                .admit_image(
                    &committed(&thumbnail, &reconciliation, "thumbnail-object"),
                    &thumbnail,
                    &reconciliation,
                    presentation(),
                    42,
                )
                .unwrap(),
            PersistentGalleryAdmissionOutcome::ThumbnailInserted
        );
        assert_eq!(
            admission
                .admit_image(
                    &committed(&original, &reconciliation, "original-object"),
                    &original,
                    &reconciliation,
                    presentation(),
                    43,
                )
                .unwrap(),
            PersistentGalleryAdmissionOutcome::OriginalAttached
        );

        let restarted = GalleryCatalogueStore::new(store.path())
            .load_or_empty()
            .unwrap();
        assert_eq!(restarted.items().len(), 1);
        let item = &restarted.items()[0];
        assert_eq!(item.review_state, GalleryReviewState::New);
        assert_eq!(item.width, 1920);
        assert_eq!(item.thumbnail.object_key, "thumbnail-object");
        assert_eq!(item.thumbnail.object_version, 7);
        assert_eq!(item.preview.as_ref().unwrap().object_key, "original-object");
        assert_eq!(item.preview.as_ref().unwrap().object_version, 7);
        assert_eq!(
            item.preview.as_ref().unwrap().availability,
            GalleryObjectAvailability::Ready
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_uncommitted_original_first_destination_change_and_conflicting_replay() {
        let (root, store) = temporary_store();
        let reconciliation = ReconciliationCatalogue::default();
        let thumbnail = plan(CaptureKind::ObservedThumbnail, "thumbnail", 320, 200);
        let original = plan(CaptureKind::ExplicitOriginal, "original", 1920, 1080);
        let mut admission = PersistentWebsiteGalleryAdmission::new(store.clone());
        assert!(matches!(
            admission.admit_image(
                &committed(&original, &reconciliation, "original-object"),
                &original,
                &reconciliation,
                presentation(),
                1,
            ),
            Err(PersistentGalleryAdmissionError::OriginalRequiresThumbnail)
        ));
        let identity =
            WebsiteCaptureReviewAdmission::canonical_media_identity(&thumbnail, &reconciliation);
        let uncommitted = Acquisition::discovered(identity).unwrap();
        assert!(matches!(
            admission.admit_image(&uncommitted, &thumbnail, &reconciliation, presentation(), 1,),
            Err(PersistentGalleryAdmissionError::Capture(_))
        ));
        admission
            .admit_image(
                &committed(&thumbnail, &reconciliation, "thumbnail-object"),
                &thumbnail,
                &reconciliation,
                presentation(),
                2,
            )
            .unwrap();
        assert!(matches!(
            admission.admit_image(
                &committed_at(
                    &original,
                    &reconciliation,
                    "another-endpoint",
                    "another-store",
                    "original-object",
                ),
                &original,
                &reconciliation,
                presentation(),
                3,
            ),
            Err(PersistentGalleryAdmissionError::DestinationMismatch)
        ));
        assert!(matches!(
            admission.admit_image(
                &committed(&thumbnail, &reconciliation, "changed-object"),
                &thumbnail,
                &reconciliation,
                presentation(),
                2,
            ),
            Err(PersistentGalleryAdmissionError::ConflictingReplay)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
