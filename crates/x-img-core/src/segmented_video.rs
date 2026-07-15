// SPDX-License-Identifier: MPL-2.0
//! Fail-closed capability gate for HLS/DASH substitution.
//!
//! This module is metadata-only. It never discovers a manifest, fetches a
//! segment, rewrites a playlist, handles encryption, or reads browser state.

#![allow(missing_docs)]

use crate::video_profile::NormalizedVideoState;

pub const SEGMENTED_GATE_SCHEMA: &str = "x-img.segmented-video-gate.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    Hls,
    Dash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedAdapterEvidence {
    pub adapter_id: String,
    pub adapter_version: String,
    pub origin: String,
    pub manifest_kind: ManifestKind,
    pub manifest_canonicalization_version: String,
    pub segment_canonicalization_version: String,
    pub normalized_profile_id: String,
    pub synthetic_fixture_id: String,
    pub firefox_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedGateRequest {
    pub schema_version: String,
    pub origin: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub manifest_kind: ManifestKind,
    pub displayed_by_user: bool,
    pub explicitly_opened: bool,
    pub drm_or_encrypted: bool,
    pub normalized_state: NormalizedVideoState,
    pub normalized_profile_id: String,
    pub evidence: Option<SegmentedAdapterEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentedGateReason {
    InvalidRequest,
    NotDisplayedOrOpened,
    DrmOrEncryption,
    AdapterEvidenceMissing,
    AdapterEvidenceMismatch,
    NormalizedRenditionNotReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentedGateDecision {
    AdapterApproved,
    OriginServed(SegmentedGateReason),
}

#[must_use]
pub fn evaluate_segmented_substitution(request: &SegmentedGateRequest) -> SegmentedGateDecision {
    if request.schema_version != SEGMENTED_GATE_SCHEMA
        || !https_origin(&request.origin)
        || !identifier(&request.adapter_id)
        || !semver(&request.adapter_version)
        || !identifier(&request.normalized_profile_id)
    {
        return SegmentedGateDecision::OriginServed(SegmentedGateReason::InvalidRequest);
    }
    if !request.displayed_by_user || !request.explicitly_opened {
        return SegmentedGateDecision::OriginServed(SegmentedGateReason::NotDisplayedOrOpened);
    }
    if request.drm_or_encrypted {
        return SegmentedGateDecision::OriginServed(SegmentedGateReason::DrmOrEncryption);
    }
    if request.normalized_state != NormalizedVideoState::Ready {
        return SegmentedGateDecision::OriginServed(
            SegmentedGateReason::NormalizedRenditionNotReady,
        );
    }
    let Some(evidence) = &request.evidence else {
        return SegmentedGateDecision::OriginServed(SegmentedGateReason::AdapterEvidenceMissing);
    };
    if evidence.adapter_id != request.adapter_id
        || evidence.adapter_version != request.adapter_version
        || evidence.origin != request.origin
        || evidence.manifest_kind != request.manifest_kind
        || evidence.normalized_profile_id != request.normalized_profile_id
        || !semver(&evidence.manifest_canonicalization_version)
        || !semver(&evidence.segment_canonicalization_version)
        || !identifier(&evidence.synthetic_fixture_id)
        || !identifier(&evidence.firefox_evidence_id)
    {
        return SegmentedGateDecision::OriginServed(SegmentedGateReason::AdapterEvidenceMismatch);
    }
    SegmentedGateDecision::AdapterApproved
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn semver(value: &str) -> bool {
    let mut parts = value.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(a), Some(b), Some(c), None)
            if [a, b, c].iter().all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    )
}

fn https_origin(value: &str) -> bool {
    value.strip_prefix("https://").is_some_and(|host| {
        !host.is_empty() && !host.contains(['/', '@', '?', '#', '*', ' ', '\n', '\r'])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SegmentedGateRequest {
        SegmentedGateRequest {
            schema_version: SEGMENTED_GATE_SCHEMA.into(),
            origin: "https://media.example.invalid".into(),
            adapter_id: "synthetic-hls".into(),
            adapter_version: "1.2.0".into(),
            manifest_kind: ManifestKind::Hls,
            displayed_by_user: true,
            explicitly_opened: true,
            drm_or_encrypted: false,
            normalized_state: NormalizedVideoState::Ready,
            normalized_profile_id: "pinakotheke-video-mp4-v1".into(),
            evidence: Some(SegmentedAdapterEvidence {
                adapter_id: "synthetic-hls".into(),
                adapter_version: "1.2.0".into(),
                origin: "https://media.example.invalid".into(),
                manifest_kind: ManifestKind::Hls,
                manifest_canonicalization_version: "1.0.0".into(),
                segment_canonicalization_version: "1.0.0".into(),
                normalized_profile_id: "pinakotheke-video-mp4-v1".into(),
                synthetic_fixture_id: "fixture-hls-1".into(),
                firefox_evidence_id: "firefox-hls-1".into(),
            }),
        }
    }

    #[test]
    fn approves_only_exact_versioned_adapter_and_normalized_evidence() {
        assert_eq!(
            evaluate_segmented_substitution(&request()),
            SegmentedGateDecision::AdapterApproved
        );
        let mut mismatched = request();
        mismatched.adapter_version = "1.3.0".into();
        assert_eq!(
            evaluate_segmented_substitution(&mismatched),
            SegmentedGateDecision::OriginServed(SegmentedGateReason::AdapterEvidenceMismatch)
        );
    }

    #[test]
    fn origin_serves_unproven_unopened_drm_and_non_ready_media() {
        let mut unproven = request();
        unproven.evidence = None;
        assert_eq!(
            evaluate_segmented_substitution(&unproven),
            SegmentedGateDecision::OriginServed(SegmentedGateReason::AdapterEvidenceMissing)
        );
        let mut unopened = request();
        unopened.explicitly_opened = false;
        assert_eq!(
            evaluate_segmented_substitution(&unopened),
            SegmentedGateDecision::OriginServed(SegmentedGateReason::NotDisplayedOrOpened)
        );
        let mut drm = request();
        drm.drm_or_encrypted = true;
        assert_eq!(
            evaluate_segmented_substitution(&drm),
            SegmentedGateDecision::OriginServed(SegmentedGateReason::DrmOrEncryption)
        );
        let mut source_only = request();
        source_only.normalized_state = NormalizedVideoState::AwaitingFirefoxPlayback;
        assert_eq!(
            evaluate_segmented_substitution(&source_only),
            SegmentedGateDecision::OriginServed(SegmentedGateReason::NormalizedRenditionNotReady)
        );
    }
}
