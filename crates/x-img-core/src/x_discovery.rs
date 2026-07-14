// SPDX-License-Identifier: MPL-2.0
//! Incremental, fixture-driven X media-discovery planning.
//!
//! This is a pure metadata boundary. It neither calls X nor downloads media;
//! a future approved adapter may provide pages only after ADR 0002 is closed.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::reconciliation::ReconciliationRequest;

pub const X_DISCOVERY_FIXTURE_SCHEMA: &str = "x-img.x-discovery-fixtures.v1";
pub const X_DISCOVERY_ADAPTER_VERSION: &str = "x-img.x-discovery.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XMediaKind {
    Photo,
    Video,
    AnimatedGif,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct XMediaVariant {
    pub variant_id: String,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub bitrate: u64,
    pub source_url_alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct XDiscoveredMedia {
    pub media_id: String,
    pub kind: XMediaKind,
    pub expected_checksum_sha256: String,
    pub variants: Vec<XMediaVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct XDiscoveredItem {
    pub item_id: String,
    pub media: Vec<XDiscoveredMedia>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct XDiscoveryPage {
    pub request_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub timeline_depth: u32,
    pub items: Vec<XDiscoveredItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XDiscoveryBudget {
    pub max_pages: u32,
    pub max_items: u32,
    pub max_timeline_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XMediaCandidate {
    pub account_id: String,
    pub item_id: String,
    pub media_id: String,
    pub canonical_media_identity: String,
    pub media_kind: XMediaKind,
    pub selected_variant: XMediaVariant,
    pub expected_checksum_sha256: String,
    pub discovery_time_unix_seconds: u64,
    pub adapter_version: String,
    pub policy_result: String,
}

impl XMediaCandidate {
    /// Converts bounded discovery metadata into the existing idempotency input.
    pub fn reconciliation_request(&self) -> Result<ReconciliationRequest, XDiscoveryError> {
        ReconciliationRequest::new(
            self.canonical_media_identity.clone(),
            self.expected_checksum_sha256.clone(),
            [self.selected_variant.source_url_alias.clone()],
        )
        .map_err(|_| XDiscoveryError::InvalidMetadata("reconciliation_request"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XDiscoveryResult {
    pub candidates: Vec<XMediaCandidate>,
    pub next_cursor: Option<String>,
    pub truncated_by_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XDiscoveryError {
    InvalidMetadata(&'static str),
    InvalidCursorChain,
    TimelineDepthExceeded,
}

impl std::fmt::Display for XDiscoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "X discovery rejected: {self:?}")
    }
}
impl std::error::Error for XDiscoveryError {}

/// Plans bounded incremental discovery from pages supplied by a future adapter.
pub fn plan_incremental_discovery(
    account_id: &str,
    pages: &[XDiscoveryPage],
    budget: XDiscoveryBudget,
    discovery_time_unix_seconds: u64,
) -> Result<XDiscoveryResult, XDiscoveryError> {
    if !is_identifier(account_id) || budget.max_pages == 0 || budget.max_items == 0 {
        return Err(XDiscoveryError::InvalidMetadata("account_or_budget"));
    }
    let mut candidates = BTreeMap::new();
    let mut expected_cursor = None;
    let mut next_cursor = None;
    let mut truncated_by_budget = false;

    for (page_index, page) in pages.iter().enumerate() {
        if u32::try_from(page_index).expect("usize exceeds u32 only on impossible input")
            >= budget.max_pages
        {
            truncated_by_budget = true;
            break;
        }
        if page.request_cursor != expected_cursor {
            return Err(XDiscoveryError::InvalidCursorChain);
        }
        if page.timeline_depth > budget.max_timeline_depth {
            return Err(XDiscoveryError::TimelineDepthExceeded);
        }
        for item in &page.items {
            if !is_identifier(&item.item_id) {
                return Err(XDiscoveryError::InvalidMetadata("item_id"));
            }
            for media in &item.media {
                let candidate = candidate_from_media(
                    account_id,
                    &item.item_id,
                    media,
                    discovery_time_unix_seconds,
                )?;
                if candidates.contains_key(&candidate.canonical_media_identity) {
                    continue;
                }
                if u32::try_from(candidates.len())
                    .expect("usize exceeds u32 only on impossible input")
                    >= budget.max_items
                {
                    truncated_by_budget = true;
                    break;
                }
                candidates.insert(candidate.canonical_media_identity.clone(), candidate);
            }
            if truncated_by_budget {
                break;
            }
        }
        next_cursor = page.next_cursor.clone();
        expected_cursor = page.next_cursor.clone();
        if truncated_by_budget || page.next_cursor.is_none() {
            break;
        }
    }
    Ok(XDiscoveryResult {
        candidates: candidates.into_values().collect(),
        next_cursor,
        truncated_by_budget,
    })
}

fn candidate_from_media(
    account_id: &str,
    item_id: &str,
    media: &XDiscoveredMedia,
    discovery_time_unix_seconds: u64,
) -> Result<XMediaCandidate, XDiscoveryError> {
    if !is_identifier(&media.media_id) || !is_sha256(&media.expected_checksum_sha256) {
        return Err(XDiscoveryError::InvalidMetadata(
            "media_identity_or_checksum",
        ));
    }
    let selected_variant = media
        .variants
        .iter()
        .filter(|variant| is_supported(media.kind, variant))
        .max_by_key(|variant| {
            (
                u64::from(variant.width) * u64::from(variant.height),
                variant.bitrate,
                variant.variant_id.as_str(),
            )
        })
        .cloned()
        .ok_or(XDiscoveryError::InvalidMetadata("supported_variant"))?;
    if !is_identifier(&selected_variant.variant_id)
        || !is_safe_alias(&selected_variant.source_url_alias)
    {
        return Err(XDiscoveryError::InvalidMetadata("variant"));
    }
    Ok(XMediaCandidate {
        account_id: account_id.to_owned(),
        item_id: item_id.to_owned(),
        media_id: media.media_id.clone(),
        canonical_media_identity: format!("x:{account_id}:{item_id}:{}", media.media_id),
        media_kind: media.kind,
        selected_variant,
        expected_checksum_sha256: media.expected_checksum_sha256.clone(),
        discovery_time_unix_seconds,
        adapter_version: X_DISCOVERY_ADAPTER_VERSION.to_owned(),
        policy_result: "fixture-only-live-gate-open".to_owned(),
    })
}

fn is_supported(kind: XMediaKind, variant: &XMediaVariant) -> bool {
    match kind {
        XMediaKind::Photo => matches!(
            variant.content_type.as_str(),
            "image/jpeg" | "image/png" | "image/webp"
        ),
        XMediaKind::Video | XMediaKind::AnimatedGif => variant.content_type == "video/mp4",
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | ':' | '-')
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn is_safe_alias(value: &str) -> bool {
    value.starts_with("https://")
        && value.len() <= 2048
        && !value.contains(['@', '?', '#', ' ', '\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquisition::VerifiedObject,
        reconciliation::{AuthorityObservation, ReconciliationCatalogue, ReconciliationOutcome},
    };
    use std::collections::BTreeSet;

    #[derive(Deserialize)]
    struct Fixture {
        schema_version: String,
        account_id: String,
        budget: FixtureBudget,
        expected_candidate_identities: BTreeSet<String>,
        pages: Vec<XDiscoveryPage>,
    }
    #[derive(Deserialize)]
    struct FixtureBudget {
        max_pages: u32,
        max_items: u32,
        max_timeline_depth: u32,
    }
    fn fixture() -> Fixture {
        serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/x-discovery/v1/pages.json"
        )))
        .expect("synthetic fixture")
    }

    #[test]
    fn fixture_pages_select_supported_best_variants_with_complete_provenance() {
        let fixture = fixture();
        assert_eq!(fixture.schema_version, X_DISCOVERY_FIXTURE_SCHEMA);
        let result = plan_incremental_discovery(
            &fixture.account_id,
            &fixture.pages,
            XDiscoveryBudget {
                max_pages: fixture.budget.max_pages,
                max_items: fixture.budget.max_items,
                max_timeline_depth: fixture.budget.max_timeline_depth,
            },
            1_784_000_000,
        )
        .expect("fixture discovery");
        assert_eq!(
            result
                .candidates
                .iter()
                .map(|candidate| candidate.canonical_media_identity.clone())
                .collect::<BTreeSet<_>>(),
            fixture.expected_candidate_identities
        );
        assert_eq!(result.next_cursor, None);
        assert!(
            result
                .candidates
                .iter()
                .any(|candidate| candidate.media_kind == XMediaKind::Photo
                    && candidate.selected_variant.variant_id == "photo-large")
        );
        assert!(
            result
                .candidates
                .iter()
                .any(|candidate| candidate.media_kind == XMediaKind::Video
                    && candidate.selected_variant.variant_id == "video-1080")
        );
        assert!(
            result
                .candidates
                .iter()
                .any(|candidate| candidate.media_kind == XMediaKind::AnimatedGif
                    && candidate.selected_variant.variant_id == "gif-mp4")
        );
        assert!(
            result
                .candidates
                .iter()
                .all(|candidate| candidate.policy_result == "fixture-only-live-gate-open")
        );
    }

    #[test]
    fn cursor_budget_and_idempotency_are_bounded() {
        let fixture = fixture();
        let result = plan_incremental_discovery(
            &fixture.account_id,
            &fixture.pages,
            XDiscoveryBudget {
                max_pages: 1,
                max_items: 16,
                max_timeline_depth: 2,
            },
            1,
        )
        .expect("first page");
        assert!(result.truncated_by_budget);
        assert_eq!(result.next_cursor.as_deref(), Some("cursor-2"));

        let candidate = plan_incremental_discovery(
            &fixture.account_id,
            &fixture.pages,
            XDiscoveryBudget {
                max_pages: 2,
                max_items: 16,
                max_timeline_depth: 2,
            },
            1,
        )
        .expect("full fixture")
        .candidates
        .into_iter()
        .next()
        .expect("candidate");
        let request = candidate.reconciliation_request().expect("request");
        let object = VerifiedObject::new(
            "fixture-endpoint",
            "fixture-store",
            "fixture-object",
            candidate.expected_checksum_sha256.clone(),
        )
        .expect("object");
        let mut catalogue = ReconciliationCatalogue::default();
        assert!(matches!(
            catalogue
                .reconcile(
                    request.clone(),
                    AuthorityObservation::Verified(object.clone())
                )
                .expect("first"),
            ReconciliationOutcome::Committed { .. }
        ));
        assert!(matches!(
            catalogue
                .reconcile(request, AuthorityObservation::Verified(object))
                .expect("replay"),
            ReconciliationOutcome::AlreadyCommitted { .. }
        ));
        assert_eq!(catalogue.committed_len(), 1);
    }

    #[test]
    fn invalid_cursor_depth_and_unsupported_variants_fail_closed() {
        let fixture = fixture();
        assert_eq!(
            plan_incremental_discovery(
                &fixture.account_id,
                &fixture.pages[1..],
                XDiscoveryBudget {
                    max_pages: 2,
                    max_items: 16,
                    max_timeline_depth: 2
                },
                1
            ),
            Err(XDiscoveryError::InvalidCursorChain)
        );
        assert_eq!(
            plan_incremental_discovery(
                &fixture.account_id,
                &fixture.pages,
                XDiscoveryBudget {
                    max_pages: 2,
                    max_items: 16,
                    max_timeline_depth: 1
                },
                1
            ),
            Err(XDiscoveryError::TimelineDepthExceeded)
        );
    }
}
