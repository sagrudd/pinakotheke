// SPDX-License-Identifier: MPL-2.0
//! Verified normalized-video admission into the persistent gallery catalogue.

#![allow(missing_docs)]

use crate::{
    gallery_catalogue::{
        GalleryCatalogueStore, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
        GalleryRepresentation, GalleryRepresentationKind, GalleryReviewState, GallerySourceKind,
        GalleryStoreError, GalleryVideoMetadata,
    },
    video_profile::{
        ManagedVideoObject, NormalizedVideoRecord, NormalizedVideoState, ProfileEvidence,
        validate_normalized_video,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryVideoPresentation {
    pub catalogue_id: String,
    pub title: String,
    pub source_label: String,
    pub width: u32,
    pub height: u32,
    pub poster_content_length: u64,
    pub video_content_length: u64,
    pub discovered_at_epoch_seconds: u64,
    pub duration_millis: u64,
    pub video_codec: String,
    pub audio_codec: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentVideoGalleryOutcome {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug)]
pub enum PersistentVideoGalleryError {
    NotReady,
    InvalidPresentation,
    Store(GalleryStoreError),
    ConflictingReplay,
}

impl std::fmt::Display for PersistentVideoGalleryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "persistent video gallery admission rejected: {self:?}"
        )
    }
}

impl std::error::Error for PersistentVideoGalleryError {}

pub fn admit_ready_normalized_video(
    store: &GalleryCatalogueStore,
    record: &NormalizedVideoRecord,
    evidence: &ProfileEvidence,
    presentation: GalleryVideoPresentation,
) -> Result<PersistentVideoGalleryOutcome, PersistentVideoGalleryError> {
    if record.state != NormalizedVideoState::Ready {
        return Err(PersistentVideoGalleryError::NotReady);
    }
    validate_normalized_video(record, Some(evidence))
        .map_err(|_| PersistentVideoGalleryError::NotReady)?;
    validate_presentation(&presentation)?;
    let poster = record
        .poster
        .as_ref()
        .ok_or(PersistentVideoGalleryError::NotReady)?;
    let video = record
        .normalized_video
        .as_ref()
        .ok_or(PersistentVideoGalleryError::NotReady)?;
    let item = GalleryItem {
        catalogue_id: presentation.catalogue_id.clone(),
        title: presentation.title,
        source_label: presentation.source_label,
        source_page_url: None,
        source_kind: GallerySourceKind::Website,
        media_kind: GalleryMediaKind::NormalizedVideo,
        review_state: GalleryReviewState::New,
        discovered_at_epoch_seconds: presentation.discovered_at_epoch_seconds,
        width: presentation.width,
        height: presentation.height,
        video: Some(GalleryVideoMetadata {
            duration_millis: presentation.duration_millis,
            video_codec: presentation.video_codec,
            audio_codec: presentation.audio_codec,
            profile_id: record.profile_id.clone(),
            normalization_state: "ready".into(),
            firefox_playback_evidence_id: evidence.evidence_id.clone(),
        }),
        thumbnail: representation(
            poster,
            GalleryRepresentationKind::VideoPoster,
            presentation.poster_content_length,
            &presentation.catalogue_id,
            "thumbnail",
        ),
        preview: Some(representation(
            video,
            GalleryRepresentationKind::NormalizedVideo,
            presentation.video_content_length,
            &presentation.catalogue_id,
            "video",
        )),
    };
    let catalogue = store
        .load_or_empty()
        .map_err(PersistentVideoGalleryError::Store)?;
    let mut items = catalogue.items().to_vec();
    if let Some(existing) = items
        .iter()
        .find(|existing| existing.catalogue_id == presentation.catalogue_id)
    {
        if existing == &item {
            return Ok(PersistentVideoGalleryOutcome::AlreadyPresent);
        }
        return Err(PersistentVideoGalleryError::ConflictingReplay);
    }
    items.push(item);
    store
        .replace(items)
        .map_err(PersistentVideoGalleryError::Store)?;
    Ok(PersistentVideoGalleryOutcome::Inserted)
}

fn representation(
    object: &ManagedVideoObject,
    kind: GalleryRepresentationKind,
    content_length: u64,
    catalogue_id: &str,
    role: &str,
) -> GalleryRepresentation {
    GalleryRepresentation {
        kind,
        availability: GalleryObjectAvailability::Ready,
        endpoint_id: object.endpoint_id.clone(),
        object_store_id: object.object_store_id.clone(),
        object_key: object.object_key.clone(),
        object_version: object.object_version,
        checksum: object.checksum.clone(),
        content_type: object.content_type.clone(),
        content_length,
        delivery_path: Some(format!(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}"
        )),
    }
}

fn validate_presentation(
    presentation: &GalleryVideoPresentation,
) -> Result<(), PersistentVideoGalleryError> {
    let safe_id = !presentation.catalogue_id.is_empty()
        && presentation.catalogue_id.len() <= 128
        && presentation.catalogue_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        });
    if !safe_id
        || presentation.title.is_empty()
        || presentation.title.len() > 256
        || presentation.title.chars().any(char::is_control)
        || presentation.source_label.is_empty()
        || presentation.source_label.len() > 256
        || presentation.source_label.chars().any(char::is_control)
        || presentation.width == 0
        || presentation.height == 0
        || presentation.poster_content_length == 0
        || presentation.video_content_length == 0
        || presentation.duration_millis == 0
        || presentation.video_codec.is_empty()
        || presentation.audio_codec.is_empty()
    {
        return Err(PersistentVideoGalleryError::InvalidPresentation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        destination::ReviewedDestination,
        video_profile::{
            DockerExecutionPlan, ExecutionPlacement, NormalizedVideoState,
            PINAKOTHEKE_VIDEO_MP4_V1, ScratchAuthority, SourceRetention, VIDEO_PROFILE_SCHEMA,
        },
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn store() -> (PathBuf, GalleryCatalogueStore) {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-video-gallery-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let store = GalleryCatalogueStore::new(root.join("gallery.json"));
        (root, store)
    }

    fn object(object_type: &str, content_type: &str) -> ManagedVideoObject {
        ManagedVideoObject {
            endpoint_id: "endpoint".into(),
            object_store_id: "store".into(),
            object_key: format!("video/{object_type}"),
            object_version: 7,
            checksum: CHECKSUM.into(),
            object_type: object_type.into(),
            content_type: content_type.into(),
        }
    }

    fn evidence() -> ProfileEvidence {
        ProfileEvidence {
            evidence_id: "evidence-1".into(),
            profile_id: PINAKOTHEKE_VIDEO_MP4_V1.into(),
            firefox_version: "firefox-140".into(),
            hardware_path: "software-and-hardware".into(),
            encoder_ref: "fixture:ffmpeg-7".into(),
            quality_ref: "fixture:quality-matrix".into(),
            storage_ref: "fixture:storage-matrix".into(),
            licensing_ref: "fixture:license-review".into(),
        }
    }

    fn record(state: NormalizedVideoState) -> NormalizedVideoRecord {
        NormalizedVideoRecord {
            schema_version: VIDEO_PROFILE_SCHEMA,
            source_identity: "source-1".into(),
            profile_id: PINAKOTHEKE_VIDEO_MP4_V1.into(),
            destination: ReviewedDestination {
                endpoint_id: "endpoint".into(),
                object_store_id: "store".into(),
                object_type: "video".into(),
                selection_kind: "site_override".into(),
                reviewed_at_unix_seconds: 1,
                actor_ref: "actor".into(),
            },
            executor: DockerExecutionPlan {
                placement: ExecutionPlacement::DasObjectStoreHost {
                    executor_ref: "dasobjectstore.executor:video".into(),
                },
                image_reference: "registry://pinakotheke/ffmpeg".into(),
                image_digest: CHECKSUM.into(),
                cpu_millis_limit: 1_000,
                memory_bytes_limit: 512 * 1024 * 1024,
                scratch_bytes_limit: 2 * 1024 * 1024 * 1024,
            },
            scratch: ScratchAuthority::DasObjectStoreManaged {
                staging_ref: "dasobjectstore.staging:video-job".into(),
            },
            state,
            normalized_video: Some(object("video-normalized", "video/mp4")),
            poster: Some(object("video-poster", "image/webp")),
            subtitles: Vec::new(),
            storyboard: None,
            provenance_manifest: Some(object("video-provenance", "application/json")),
            source_retention: SourceRetention::NotRetainedByPolicy,
            firefox_playback_evidence_id: Some("evidence-1".into()),
        }
    }

    fn presentation() -> GalleryVideoPresentation {
        GalleryVideoPresentation {
            catalogue_id: "video-card-1".into(),
            title: "Redistributable normalized video".into(),
            source_label: "Website / example-site".into(),
            width: 1920,
            height: 1080,
            poster_content_length: 12,
            video_content_length: 26,
            discovered_at_epoch_seconds: 42,
            duration_millis: 12_345,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
        }
    }

    #[test]
    fn ready_video_and_poster_persist_as_one_restart_safe_card() {
        let (root, store) = store();
        assert_eq!(
            admit_ready_normalized_video(
                &store,
                &record(NormalizedVideoState::Ready),
                &evidence(),
                presentation(),
            )
            .unwrap(),
            PersistentVideoGalleryOutcome::Inserted
        );
        let restarted = GalleryCatalogueStore::new(store.path())
            .load_or_empty()
            .unwrap();
        let item = &restarted.items()[0];
        assert_eq!(item.media_kind, GalleryMediaKind::NormalizedVideo);
        assert_eq!(item.thumbnail.kind, GalleryRepresentationKind::VideoPoster);
        assert_eq!(item.thumbnail.object_version, 7);
        assert_eq!(
            item.preview.as_ref().unwrap().kind,
            GalleryRepresentationKind::NormalizedVideo
        );
        assert_eq!(item.preview.as_ref().unwrap().object_version, 7);
        assert_eq!(item.review_state, GalleryReviewState::New);
        assert_eq!(
            admit_ready_normalized_video(
                &store,
                &record(NormalizedVideoState::Ready),
                &evidence(),
                presentation(),
            )
            .unwrap(),
            PersistentVideoGalleryOutcome::AlreadyPresent
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_unproven_and_conflicting_video_cards() {
        let (root, store) = store();
        assert!(matches!(
            admit_ready_normalized_video(
                &store,
                &record(NormalizedVideoState::AwaitingFirefoxPlayback),
                &evidence(),
                presentation(),
            ),
            Err(PersistentVideoGalleryError::NotReady)
        ));
        admit_ready_normalized_video(
            &store,
            &record(NormalizedVideoState::Ready),
            &evidence(),
            presentation(),
        )
        .unwrap();
        let mut changed = presentation();
        changed.video_content_length = 27;
        assert!(matches!(
            admit_ready_normalized_video(
                &store,
                &record(NormalizedVideoState::Ready),
                &evidence(),
                changed,
            ),
            Err(PersistentVideoGalleryError::ConflictingReplay)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
