// SPDX-License-Identifier: MPL-2.0
//! Verified website-capture admission into the persistent gallery catalogue.

#![allow(missing_docs)]

use crate::{
    acquisition::Acquisition,
    gallery_catalogue::{
        GalleryCatalogueStore, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
        GalleryRepresentation, GalleryRepresentationKind, GalleryReviewState, GallerySourceKind,
        GalleryStoreError, GalleryVideoMetadata,
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
    pub video: Option<GalleryVideoCompletion>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryVideoCompletion {
    pub duration_millis: u64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub profile_id: String,
    pub firefox_playback_evidence_id: String,
    pub poster_object_key: String,
    pub poster_object_version: u64,
    pub poster_checksum_sha256: String,
    pub poster_content_length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentGalleryAdmissionOutcome {
    ThumbnailInserted,
    OriginalAttached,
    VideoInserted,
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
                CaptureKind::ExplicitVideo => GalleryRepresentationKind::NormalizedVideo,
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
                    CaptureKind::ExplicitVideo => "video",
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
        let (source_label, source_kind) = gallery_source(plan);
        let presentation_width = presentation
            .video
            .as_ref()
            .map_or(plan.width, |video| video.width);
        let presentation_height = presentation
            .video
            .as_ref()
            .map_or(plan.height, |video| video.height);
        let outcome = match (plan.capture_kind, existing) {
            (CaptureKind::ObservedThumbnail, None) => {
                items.push(GalleryItem {
                    catalogue_id: presentation.catalogue_id,
                    title: presentation.title,
                    source_label: source_label.clone(),
                    source_page_url: source_page_url(plan),
                    source_kind,
                    media_kind: GalleryMediaKind::Image,
                    review_state: GalleryReviewState::New,
                    discovered_at_epoch_seconds,
                    width: presentation_width,
                    height: presentation_height,
                    video: None,
                    thumbnail: representation,
                    preview: None,
                });
                PersistentGalleryAdmissionOutcome::ThumbnailInserted
            }
            (CaptureKind::ObservedThumbnail, Some(item)) => {
                if item.thumbnail == representation {
                    return Ok(PersistentGalleryAdmissionOutcome::AlreadyPresent);
                }
                if item.preview.as_ref().is_some_and(|preview| {
                    item.thumbnail.object_key == preview.object_key
                        && item.thumbnail.object_version == preview.object_version
                        && item.thumbnail.checksum == preview.checksum
                }) {
                    if item.thumbnail.endpoint_id != representation.endpoint_id
                        || item.thumbnail.object_store_id != representation.object_store_id
                    {
                        return Err(PersistentGalleryAdmissionError::DestinationMismatch);
                    }
                    item.thumbnail = representation;
                    PersistentGalleryAdmissionOutcome::ThumbnailInserted
                } else {
                    return Err(PersistentGalleryAdmissionError::ConflictingReplay);
                }
            }
            (CaptureKind::ExplicitOriginal, None) => {
                let mut thumbnail = representation.clone();
                thumbnail.kind = GalleryRepresentationKind::Thumbnail;
                thumbnail.delivery_path = Some(format!(
                    "/products/pinakotheke/api/gallery/v1/objects/{}/thumbnail",
                    presentation.catalogue_id
                ));
                items.push(GalleryItem {
                    catalogue_id: presentation.catalogue_id,
                    title: presentation.title,
                    source_label,
                    source_page_url: source_page_url(plan),
                    source_kind,
                    media_kind: GalleryMediaKind::Image,
                    review_state: GalleryReviewState::New,
                    discovered_at_epoch_seconds,
                    width: presentation_width,
                    height: presentation_height,
                    video: None,
                    thumbnail,
                    preview: Some(representation),
                });
                PersistentGalleryAdmissionOutcome::OriginalAttached
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
                if let Some(source_page_url) = source_page_url(plan) {
                    item.source_page_url = Some(source_page_url);
                }
                item.width = plan.width;
                item.height = plan.height;
                PersistentGalleryAdmissionOutcome::OriginalAttached
            }
            (CaptureKind::ExplicitVideo, None) => {
                let (poster, video) = if let Some(video) = presentation.video {
                    let poster = GalleryRepresentation {
                        kind: GalleryRepresentationKind::VideoPoster,
                        availability: GalleryObjectAvailability::Ready,
                        endpoint_id: representation.endpoint_id.clone(),
                        object_store_id: representation.object_store_id.clone(),
                        object_key: video.poster_object_key.clone(),
                        object_version: video.poster_object_version,
                        checksum: format!("sha256:{}", video.poster_checksum_sha256),
                        content_type: "image/webp".into(),
                        content_length: video.poster_content_length,
                        delivery_path: Some(format!(
                            "/products/pinakotheke/api/gallery/v1/objects/{}/thumbnail",
                            presentation.catalogue_id
                        )),
                    };
                    let metadata = GalleryVideoMetadata {
                        duration_millis: video.duration_millis,
                        video_codec: video.video_codec,
                        audio_codec: video.audio_codec,
                        profile_id: video.profile_id,
                        normalization_state: "ready".into(),
                        firefox_playback_evidence_id: video.firefox_playback_evidence_id,
                    };
                    (poster, Some(metadata))
                } else {
                    let mut placeholder = representation.clone();
                    placeholder.kind = GalleryRepresentationKind::VideoPoster;
                    placeholder.availability = GalleryObjectAvailability::Unavailable;
                    placeholder.delivery_path = None;
                    (placeholder, None)
                };
                items.push(GalleryItem {
                    catalogue_id: presentation.catalogue_id,
                    title: presentation.title,
                    source_label,
                    source_page_url: source_page_url(plan),
                    source_kind,
                    media_kind: GalleryMediaKind::NormalizedVideo,
                    review_state: GalleryReviewState::New,
                    discovered_at_epoch_seconds,
                    width: presentation_width,
                    height: presentation_height,
                    video,
                    thumbnail: poster,
                    preview: Some(representation),
                });
                PersistentGalleryAdmissionOutcome::VideoInserted
            }
            (CaptureKind::ExplicitVideo, Some(item)) => {
                if item.preview.as_ref() == Some(&representation) {
                    return Ok(PersistentGalleryAdmissionOutcome::AlreadyPresent);
                }
                return Err(PersistentGalleryAdmissionError::ConflictingReplay);
            }
        };
        self.store
            .replace(items)
            .map_err(PersistentGalleryAdmissionError::Store)?;
        Ok(outcome)
    }
}

fn gallery_source(plan: &CapturePlan) -> (String, GallerySourceKind) {
    for candidate in [&plan.canonical_presentation_url, &plan.canonical_page_url] {
        if let Some(account) = candidate
            .strip_prefix("https://x.com/")
            .and_then(|rest| rest.split(['/', '?', '#']).next())
            .filter(|account| {
                !account.is_empty()
                    && !matches!(
                        *account,
                        "home" | "i" | "search" | "explore" | "notifications"
                    )
                    && account.len() <= 15
                    && account
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            })
        {
            return (format!("X / @{account}"), GallerySourceKind::XAccount);
        }
    }
    if let Some(creator) = plan.creator_hint.as_deref() {
        return (format!("Website / {creator}"), GallerySourceKind::Website);
    }
    (
        format!("Website / {}", plan.site_id),
        GallerySourceKind::Website,
    )
}

fn source_page_url(plan: &CapturePlan) -> Option<String> {
    [&plan.canonical_presentation_url, &plan.canonical_page_url]
        .into_iter()
        .find_map(|candidate| {
            let mut url = candidate.parse::<url::Url>().ok()?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return None;
            }
            url.set_query(None);
            url.set_fragment(None);
            Some(url.to_string())
        })
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
        || !(presentation.content_type.starts_with("image/")
            || presentation.content_type == "video/mp4")
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
            retrieval_media_url: format!("https://example.invalid/{media}.jpg"),
            destination: None,
            canonical_presentation_url: format!("https://example.invalid/{media}.jpg"),
            creator_hint: None,
            catalogue_id: "gallery-image-1".into(),
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
            video: None,
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
        assert_eq!(
            item.source_page_url.as_deref(),
            Some("https://example.invalid/original.jpg")
        );
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
    fn verified_firefox_compatible_video_is_immediately_browsable() {
        let (root, store) = temporary_store();
        let reconciliation = ReconciliationCatalogue::default();
        let mut video = plan(CaptureKind::ExplicitVideo, "video", 1280, 720);
        video.canonical_media_url = "https://video.twimg.com/ext_tw_video/fixture/video.mp4".into();
        video.catalogue_id = "gallery-video-1".into();
        let presentation = GalleryImagePresentation {
            catalogue_id: "gallery-video-1".into(),
            title: "Captured video from x-web".into(),
            content_type: "video/mp4".into(),
            content_length: 42,
            video: Some(GalleryVideoCompletion {
                duration_millis: 12_345,
                width: 1920,
                height: 1080,
                video_codec: "h264".into(),
                audio_codec: "aac".into(),
                profile_id: "pinakotheke-video-mp4-v1".into(),
                firefox_playback_evidence_id: "firefox-140-linux".into(),
                poster_object_key: "x.com/artist/status/poster.webp".into(),
                poster_object_version: 8,
                poster_checksum_sha256: "b".repeat(64),
                poster_content_length: 24,
            }),
        };
        let mut admission = PersistentWebsiteGalleryAdmission::new(store.clone());
        assert_eq!(
            admission
                .admit_image(
                    &committed(&video, &reconciliation, "video-object"),
                    &video,
                    &reconciliation,
                    presentation,
                    42,
                )
                .unwrap(),
            PersistentGalleryAdmissionOutcome::VideoInserted
        );
        let catalogue = store.load_or_empty().unwrap();
        let item = &catalogue.items()[0];
        assert_eq!(item.media_kind, GalleryMediaKind::NormalizedVideo);
        assert_eq!(
            item.thumbnail.availability,
            GalleryObjectAvailability::Ready
        );
        assert_eq!(item.thumbnail.content_type, "image/webp");
        assert_eq!(item.preview.as_ref().unwrap().content_type, "video/mp4");
        assert_eq!((item.width, item.height), (1920, 1080));
        assert_eq!(item.video.as_ref().unwrap().duration_millis, 12_345);
        assert_eq!(item.video.as_ref().unwrap().normalization_state, "ready");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x_presentation_records_the_creator_as_gallery_source() {
        let mut capture = plan(CaptureKind::ObservedThumbnail, "thumb", 320, 200);
        capture.canonical_page_url = "https://x.com/home".into();
        capture.canonical_presentation_url =
            "https://x.com/Fixture_Artist/status/123/photo/1".into();
        assert_eq!(
            gallery_source(&capture),
            ("X / @Fixture_Artist".into(), GallerySourceKind::XAccount)
        );
    }

    #[test]
    fn admits_original_first_and_replaces_its_renderable_fallback_with_a_thumbnail() {
        let (root, store) = temporary_store();
        let reconciliation = ReconciliationCatalogue::default();
        let thumbnail = plan(CaptureKind::ObservedThumbnail, "thumbnail", 320, 200);
        let original = plan(CaptureKind::ExplicitOriginal, "original", 1920, 1080);
        let mut admission = PersistentWebsiteGalleryAdmission::new(store.clone());
        assert_eq!(
            admission
                .admit_image(
                    &committed(&original, &reconciliation, "original-object"),
                    &original,
                    &reconciliation,
                    presentation(),
                    1,
                )
                .unwrap(),
            PersistentGalleryAdmissionOutcome::OriginalAttached
        );
        let original_first = store.load_or_empty().unwrap();
        assert_eq!(original_first.items().len(), 1);
        assert_eq!(
            original_first.items()[0].thumbnail.object_key,
            "original-object"
        );
        assert_eq!(
            original_first.items()[0]
                .preview
                .as_ref()
                .unwrap()
                .object_key,
            "original-object"
        );
        assert_eq!(
            admission
                .admit_image(
                    &committed(&thumbnail, &reconciliation, "thumbnail-object"),
                    &thumbnail,
                    &reconciliation,
                    presentation(),
                    2,
                )
                .unwrap(),
            PersistentGalleryAdmissionOutcome::ThumbnailInserted
        );
        let enriched = store.load_or_empty().unwrap();
        assert_eq!(enriched.items()[0].thumbnail.object_key, "thumbnail-object");
        assert_eq!(
            enriched.items()[0].preview.as_ref().unwrap().object_key,
            "original-object"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_uncommitted_destination_change_and_conflicting_replay() {
        let (root, store) = temporary_store();
        let reconciliation = ReconciliationCatalogue::default();
        let thumbnail = plan(CaptureKind::ObservedThumbnail, "thumbnail", 320, 200);
        let original = plan(CaptureKind::ExplicitOriginal, "original", 1920, 1080);
        let mut admission = PersistentWebsiteGalleryAdmission::new(store.clone());
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
