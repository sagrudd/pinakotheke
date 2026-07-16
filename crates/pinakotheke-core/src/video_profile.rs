// SPDX-License-Identifier: MPL-2.0
//! Versioned normalized-video profile and Docker execution-placement contracts.
//!
//! This module is metadata only. It neither invokes Docker nor moves video
//! bytes. A later worker must use the selected container digest, authorized
//! execution placement, bounded scratch, and DASObjectStore authority.

#![allow(missing_docs)]

use crate::destination::ReviewedDestination;

pub const VIDEO_PROFILE_SCHEMA: &str = "x-img.normalized-video.v1";
pub const PINAKOTHEKE_VIDEO_WEBM_V1: &str = "pinakotheke-video-webm-v1";
pub const PINAKOTHEKE_VIDEO_MP4_V1: &str = "pinakotheke-video-mp4-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoContainer {
    Webm,
    Mp4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    Vp9,
    Av1,
    H264,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    Opus,
    Aac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecVariant {
    pub video: VideoCodec,
    pub audio: AudioCodec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackProfile {
    pub profile_id: &'static str,
    pub container: VideoContainer,
    pub content_type: &'static str,
    pub variants: &'static [CodecVariant],
}

const WEBM_VARIANTS: &[CodecVariant] = &[
    CodecVariant {
        video: VideoCodec::Vp9,
        audio: AudioCodec::Opus,
    },
    CodecVariant {
        video: VideoCodec::Av1,
        audio: AudioCodec::Opus,
    },
];
const MP4_VARIANTS: &[CodecVariant] = &[CodecVariant {
    video: VideoCodec::H264,
    audio: AudioCodec::Aac,
}];
const PROFILES: &[PlaybackProfile] = &[
    PlaybackProfile {
        profile_id: PINAKOTHEKE_VIDEO_WEBM_V1,
        container: VideoContainer::Webm,
        content_type: "video/webm",
        variants: WEBM_VARIANTS,
    },
    PlaybackProfile {
        profile_id: PINAKOTHEKE_VIDEO_MP4_V1,
        container: VideoContainer::Mp4,
        content_type: "video/mp4",
        variants: MP4_VARIANTS,
    },
];

#[must_use]
pub const fn playback_profiles() -> &'static [PlaybackProfile] {
    PROFILES
}

#[must_use]
pub fn playback_profile(profile_id: &str) -> Option<&'static PlaybackProfile> {
    PROFILES
        .iter()
        .find(|profile| profile.profile_id == profile_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileEvidence {
    pub evidence_id: String,
    pub profile_id: String,
    pub firefox_version: String,
    pub hardware_path: String,
    pub encoder_ref: String,
    pub quality_ref: String,
    pub storage_ref: String,
    pub licensing_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPlacement {
    DasObjectStoreHost {
        executor_ref: String,
    },
    PairedFirefoxDevice {
        pairing_ref: String,
        device_ref: String,
    },
    KeryxWorker {
        dispatch_ref: String,
        worker_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerExecutionPlan {
    pub placement: ExecutionPlacement,
    pub image_reference: String,
    pub image_digest: String,
    pub cpu_millis_limit: u32,
    pub memory_bytes_limit: u64,
    pub scratch_bytes_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScratchAuthority {
    DasObjectStoreManaged { staging_ref: String },
    BoundedEphemeral { cleanup_ref: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedVideoObject {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub object_version: u64,
    pub checksum: String,
    pub object_type: String,
    pub content_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRetention {
    NotRetainedByPolicy,
    Retained(ManagedVideoObject),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedVideoState {
    Planned,
    Normalizing,
    AwaitingFirefoxPlayback,
    Ready,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedVideoRecord {
    pub schema_version: &'static str,
    pub source_identity: String,
    pub profile_id: String,
    pub destination: ReviewedDestination,
    pub executor: DockerExecutionPlan,
    pub scratch: ScratchAuthority,
    pub state: NormalizedVideoState,
    pub normalized_video: Option<ManagedVideoObject>,
    pub poster: Option<ManagedVideoObject>,
    pub subtitles: Vec<ManagedVideoObject>,
    pub storyboard: Option<ManagedVideoObject>,
    pub provenance_manifest: Option<ManagedVideoObject>,
    pub source_retention: SourceRetention,
    pub firefox_playback_evidence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoProfileError {
    Invalid(String),
    UnknownProfile,
    EvidenceMissing,
    SourceOnlyCannotBeReady,
}

impl std::fmt::Display for VideoProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => {
                write!(formatter, "invalid normalized-video contract: {message}")
            }
            Self::UnknownProfile => formatter.write_str("normalized-video profile is unknown"),
            Self::EvidenceMissing => {
                formatter.write_str("required profile or Firefox evidence is missing")
            }
            Self::SourceOnlyCannotBeReady => {
                formatter.write_str("a source-only video cannot be marked ready")
            }
        }
    }
}

impl std::error::Error for VideoProfileError {}

pub fn validate_profile_evidence(evidence: &ProfileEvidence) -> Result<(), VideoProfileError> {
    if playback_profile(&evidence.profile_id).is_none() {
        return Err(VideoProfileError::UnknownProfile);
    }
    for value in [
        evidence.evidence_id.as_str(),
        evidence.firefox_version.as_str(),
        evidence.hardware_path.as_str(),
        evidence.encoder_ref.as_str(),
        evidence.quality_ref.as_str(),
        evidence.storage_ref.as_str(),
        evidence.licensing_ref.as_str(),
    ] {
        if !non_secret_reference(value) {
            return Err(VideoProfileError::Invalid(
                "evidence fields must be bounded non-secret references".into(),
            ));
        }
    }
    Ok(())
}

pub fn validate_normalized_video(
    record: &NormalizedVideoRecord,
    evidence: Option<&ProfileEvidence>,
) -> Result<(), VideoProfileError> {
    if record.schema_version != VIDEO_PROFILE_SCHEMA || !identifier(&record.source_identity) {
        return Err(VideoProfileError::Invalid(
            "schema version or source identity is invalid".into(),
        ));
    }
    let profile = playback_profile(&record.profile_id).ok_or(VideoProfileError::UnknownProfile)?;
    validate_destination(&record.destination)?;
    validate_executor(&record.executor)?;
    validate_scratch(&record.scratch)?;
    validate_retained_source(&record.source_retention, &record.destination)?;
    for object in record
        .normalized_video
        .iter()
        .chain(record.poster.iter())
        .chain(record.subtitles.iter())
        .chain(record.storyboard.iter())
        .chain(record.provenance_manifest.iter())
    {
        validate_object(object, &record.destination)?;
    }
    if let Some(video) = &record.normalized_video
        && video.content_type != profile.content_type
    {
        return Err(VideoProfileError::Invalid(
            "normalized video content type disagrees with its profile".into(),
        ));
    }
    if record.state == NormalizedVideoState::Ready {
        let evidence = evidence.ok_or(VideoProfileError::EvidenceMissing)?;
        validate_profile_evidence(evidence)?;
        if evidence.profile_id != record.profile_id
            || !record
                .firefox_playback_evidence_id
                .as_ref()
                .is_some_and(|id| id == &evidence.evidence_id)
        {
            return Err(VideoProfileError::EvidenceMissing);
        }
        if record.normalized_video.is_none()
            || record.poster.is_none()
            || record.provenance_manifest.is_none()
        {
            return Err(VideoProfileError::SourceOnlyCannotBeReady);
        }
        if record
            .normalized_video
            .as_ref()
            .is_none_or(|object| object.object_type != "video-normalized")
            || record
                .poster
                .as_ref()
                .is_none_or(|object| object.object_type != "video-poster")
            || record
                .provenance_manifest
                .as_ref()
                .is_none_or(|object| object.object_type != "video-provenance")
            || record
                .subtitles
                .iter()
                .any(|object| object.object_type != "video-subtitle")
            || record
                .storyboard
                .as_ref()
                .is_some_and(|object| object.object_type != "video-storyboard")
        {
            return Err(VideoProfileError::Invalid(
                "derived objects must retain their normalized-video types".into(),
            ));
        }
    }
    Ok(())
}

fn validate_destination(destination: &ReviewedDestination) -> Result<(), VideoProfileError> {
    if !identifier(&destination.endpoint_id)
        || !identifier(&destination.object_store_id)
        || destination.object_type != "video"
    {
        return Err(VideoProfileError::Invalid(
            "destination must be an identified video ObjectStore".into(),
        ));
    }
    Ok(())
}

fn validate_executor(executor: &DockerExecutionPlan) -> Result<(), VideoProfileError> {
    let placement_valid = match &executor.placement {
        ExecutionPlacement::DasObjectStoreHost { executor_ref } => {
            executor_ref.starts_with("dasobjectstore.executor:")
        }
        ExecutionPlacement::PairedFirefoxDevice {
            pairing_ref,
            device_ref,
        } => pairing_ref.starts_with("x-img.pairing:") && device_ref.starts_with("device:"),
        ExecutionPlacement::KeryxWorker {
            dispatch_ref,
            worker_ref,
        } => dispatch_ref.starts_with("keryx.dispatch:") && worker_ref.starts_with("keryx.worker:"),
    };
    if !placement_valid
        || !executor.image_reference.starts_with("registry://")
        || !sha256(&executor.image_digest)
        || executor.cpu_millis_limit == 0
        || executor.memory_bytes_limit == 0
        || executor.scratch_bytes_limit == 0
    {
        return Err(VideoProfileError::Invalid(
            "Docker executor placement, image digest, or resource bounds are invalid".into(),
        ));
    }
    Ok(())
}

fn validate_scratch(scratch: &ScratchAuthority) -> Result<(), VideoProfileError> {
    let valid = match scratch {
        ScratchAuthority::DasObjectStoreManaged { staging_ref } => {
            staging_ref.starts_with("dasobjectstore.staging:")
        }
        ScratchAuthority::BoundedEphemeral { cleanup_ref } => {
            cleanup_ref.starts_with("x-img.scratch-cleanup:")
        }
    };
    valid.then_some(()).ok_or_else(|| {
        VideoProfileError::Invalid("scratch authority must be managed or bounded".into())
    })
}

fn validate_retained_source(
    source: &SourceRetention,
    destination: &ReviewedDestination,
) -> Result<(), VideoProfileError> {
    if let SourceRetention::Retained(source) = source {
        validate_object(source, destination)?;
        if source.object_type != "video-source" {
            return Err(VideoProfileError::Invalid(
                "retained source must be a typed video-source object".into(),
            ));
        }
    }
    Ok(())
}

fn validate_object(
    object: &ManagedVideoObject,
    destination: &ReviewedDestination,
) -> Result<(), VideoProfileError> {
    if object.endpoint_id != destination.endpoint_id
        || object.object_store_id != destination.object_store_id
        || !safe_object_key(&object.object_key)
        || object.object_version == 0
        || !sha256(&object.checksum)
        || object.object_type.is_empty()
        || object.content_type.is_empty()
    {
        return Err(VideoProfileError::Invalid(
            "derived object must be a checksummed typed object in the reviewed destination".into(),
        ));
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn safe_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains(['?', '#', '@', '\\', '\n', '\r'])
}

fn sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn non_secret_reference(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.contains(['?', '#', '@', '\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn destination() -> ReviewedDestination {
        ReviewedDestination {
            endpoint_id: "endpoint".into(),
            object_store_id: "store".into(),
            object_type: "video".into(),
            selection_kind: "site_override".into(),
            reviewed_at_unix_seconds: 1,
            actor_ref: "actor".into(),
        }
    }

    fn object(object_type: &str, content_type: &str) -> ManagedVideoObject {
        ManagedVideoObject {
            endpoint_id: "endpoint".into(),
            object_store_id: "store".into(),
            object_key: format!("video/{object_type}"),
            object_version: 1,
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

    fn ready_record() -> NormalizedVideoRecord {
        NormalizedVideoRecord {
            schema_version: VIDEO_PROFILE_SCHEMA,
            source_identity: "source-1".into(),
            profile_id: PINAKOTHEKE_VIDEO_MP4_V1.into(),
            destination: destination(),
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
            state: NormalizedVideoState::Ready,
            normalized_video: Some(object("video-normalized", "video/mp4")),
            poster: Some(object("video-poster", "image/webp")),
            subtitles: vec![object("video-subtitle", "text/vtt")],
            storyboard: Some(object("video-storyboard", "image/webp")),
            provenance_manifest: Some(object("video-provenance", "application/json")),
            source_retention: SourceRetention::NotRetainedByPolicy,
            firefox_playback_evidence_id: Some("evidence-1".into()),
        }
    }

    #[test]
    fn profiles_cover_vp9_opus_av1_opus_and_h264_aac_without_a_default() {
        assert_eq!(playback_profiles().len(), 2);
        assert_eq!(playback_profiles()[0].variants, WEBM_VARIANTS);
        assert_eq!(playback_profiles()[1].variants, MP4_VARIANTS);
    }

    #[test]
    fn ready_requires_normalized_typed_objects_and_matching_firefox_evidence() {
        let record = ready_record();
        assert!(validate_normalized_video(&record, Some(&evidence())).is_ok());

        let mut source_only = record;
        source_only.normalized_video = None;
        assert_eq!(
            validate_normalized_video(&source_only, Some(&evidence())),
            Err(VideoProfileError::SourceOnlyCannotBeReady)
        );

        let mut wrong_type = ready_record();
        wrong_type.poster.as_mut().expect("poster").object_type = "image".into();
        assert!(matches!(
            validate_normalized_video(&wrong_type, Some(&evidence())),
            Err(VideoProfileError::Invalid(_))
        ));
    }

    #[test]
    fn accepts_only_pinned_docker_on_an_authorized_host_placement() {
        let mut record = ready_record();
        record.executor.placement = ExecutionPlacement::KeryxWorker {
            dispatch_ref: "keryx.dispatch:approved-job".into(),
            worker_ref: "keryx.worker:video".into(),
        };
        record.scratch = ScratchAuthority::BoundedEphemeral {
            cleanup_ref: "x-img.scratch-cleanup:job".into(),
        };
        assert!(validate_normalized_video(&record, Some(&evidence())).is_ok());

        record.executor.image_digest = "latest".into();
        assert!(matches!(
            validate_normalized_video(&record, Some(&evidence())),
            Err(VideoProfileError::Invalid(_))
        ));
    }
}
