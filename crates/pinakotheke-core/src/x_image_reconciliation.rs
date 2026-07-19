// SPDX-License-Identifier: MPL-2.0
//! Guarded metadata-only reconciliation for historic X image gallery identities.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    capture_plan_journal::PendingCapturePlan,
    gallery_catalogue::{
        GalleryItem, GalleryMediaKind, GalleryObjectAvailability, GalleryReviewState,
        GallerySourceKind,
    },
    viewed_media::{CaptureKind, x_image_catalogue_id},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XImageReconciliationReport {
    pub duplicate_groups: usize,
    pub redundant_cards: usize,
    pub renamed_cards: usize,
    pub rebound_plans: usize,
}

impl XImageReconciliationReport {
    #[must_use]
    pub const fn changed(self) -> bool {
        self.redundant_cards > 0 || self.renamed_cards > 0 || self.rebound_plans > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XImageReconciliation {
    pub items: Vec<GalleryItem>,
    pub plans: Vec<PendingCapturePlan>,
    pub report: XImageReconciliationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XImageReconciliationError {
    AmbiguousHistoricCard,
    DestinationMismatch,
    TargetCollision,
}

impl std::fmt::Display for XImageReconciliationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AmbiguousHistoricCard => {
                "one historic gallery card resolves to multiple X media identities"
            }
            Self::DestinationMismatch => {
                "duplicate cards reference different endpoint/ObjectStore authorities"
            }
            Self::TargetCollision => {
                "a stable X image identity collides with an unrelated gallery card"
            }
        })
    }
}

impl std::error::Error for XImageReconciliationError {}

/// Rebind historic page-derived X image IDs to one immutable media-derived ID.
///
/// Only metadata references are changed. Every DASObjectStore object remains
/// untouched. Exact endpoint/ObjectStore agreement is required before cards
/// are merged, and the highest-fidelity ready original is retained.
pub fn reconcile_x_image_catalogue(
    items: Vec<GalleryItem>,
    mut plans: Vec<PendingCapturePlan>,
) -> Result<XImageReconciliation, XImageReconciliationError> {
    let existing_ids = items
        .iter()
        .map(|item| item.catalogue_id.clone())
        .collect::<BTreeSet<_>>();
    let mut old_to_stable = BTreeMap::<String, String>::new();
    for pending in plans.iter().filter(|pending| pending.settled) {
        if pending.plan.capture_kind == CaptureKind::ExplicitVideo {
            continue;
        }
        let Some(stable) = x_image_catalogue_id(&pending.plan.canonical_media_url) else {
            continue;
        };
        if !existing_ids.contains(&pending.plan.catalogue_id) {
            continue;
        }
        match old_to_stable.insert(pending.plan.catalogue_id.clone(), stable.clone()) {
            Some(previous) if previous != stable => {
                return Err(XImageReconciliationError::AmbiguousHistoricCard);
            }
            _ => {}
        }
    }

    let mapped_targets = old_to_stable.values().cloned().collect::<BTreeSet<_>>();
    if items.iter().any(|item| {
        mapped_targets.contains(&item.catalogue_id)
            && !old_to_stable.contains_key(&item.catalogue_id)
    }) {
        return Err(XImageReconciliationError::TargetCollision);
    }

    let mut grouped = BTreeMap::<String, Vec<GalleryItem>>::new();
    let mut reconciled_items = Vec::with_capacity(items.len());
    for item in items {
        if let Some(stable) = old_to_stable.get(&item.catalogue_id) {
            grouped.entry(stable.clone()).or_default().push(item);
        } else {
            reconciled_items.push(item);
        }
    }

    let mut report = XImageReconciliationReport::default();
    for (stable, group) in grouped {
        if group.len() > 1 {
            report.duplicate_groups += 1;
            report.redundant_cards += group.len() - 1;
        }
        report.renamed_cards += group
            .iter()
            .filter(|item| item.catalogue_id != stable)
            .count();
        reconciled_items.push(merge_group(stable, group)?);
    }

    for pending in &mut plans {
        if pending.plan.capture_kind == CaptureKind::ExplicitVideo {
            continue;
        }
        if let Some(stable) = x_image_catalogue_id(&pending.plan.canonical_media_url)
            && pending.plan.catalogue_id != stable
        {
            pending.plan.catalogue_id = stable;
            report.rebound_plans += 1;
        }
    }

    Ok(XImageReconciliation {
        items: reconciled_items,
        plans,
        report,
    })
}

fn merge_group(
    stable: String,
    group: Vec<GalleryItem>,
) -> Result<GalleryItem, XImageReconciliationError> {
    debug_assert!(!group.is_empty());
    if group
        .iter()
        .any(|item| item.media_kind != GalleryMediaKind::Image)
    {
        return Err(XImageReconciliationError::TargetCollision);
    }
    let destination = (
        group[0].thumbnail.endpoint_id.as_str(),
        group[0].thumbnail.object_store_id.as_str(),
    );
    if group.iter().any(|item| {
        std::iter::once(&item.thumbnail)
            .chain(item.preview.iter())
            .any(|representation| {
                representation.endpoint_id != destination.0
                    || representation.object_store_id != destination.1
            })
    }) {
        return Err(XImageReconciliationError::DestinationMismatch);
    }

    let preview_choice = group
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.preview.as_ref().map(|preview| (index, preview)))
        .max_by_key(|(_, preview)| {
            (
                preview.availability == GalleryObjectAvailability::Ready,
                preview.content_length,
            )
        });
    let metadata_index = preview_choice.map_or_else(
        || {
            group
                .iter()
                .enumerate()
                .max_by_key(|(_, item)| {
                    (
                        item.source_kind == GallerySourceKind::XAccount,
                        u64::from(item.width) * u64::from(item.height),
                        item.discovered_at_epoch_seconds,
                    )
                })
                .map_or(0, |(index, _)| index)
        },
        |(index, _)| index,
    );
    let thumbnail_index = group
        .iter()
        .enumerate()
        .min_by_key(|(_, item)| {
            let duplicates_preview = item.preview.as_ref().is_some_and(|preview| {
                preview.object_key == item.thumbnail.object_key
                    && preview.object_version == item.thumbnail.object_version
                    && preview.checksum == item.thumbnail.checksum
            });
            (
                item.thumbnail.availability != GalleryObjectAvailability::Ready,
                duplicates_preview,
                item.thumbnail.content_length,
            )
        })
        .map_or(0, |(index, _)| index);

    let metadata = &group[metadata_index];
    let mut thumbnail = group[thumbnail_index].thumbnail.clone();
    thumbnail.delivery_path = ready_delivery_path(thumbnail.availability, &stable, "thumbnail");
    let preview = preview_choice.map(|(_, preview)| {
        let mut preview = preview.clone();
        preview.delivery_path = ready_delivery_path(preview.availability, &stable, "original");
        preview
    });
    let review_state = if group
        .iter()
        .any(|item| item.review_state == GalleryReviewState::Removed)
    {
        GalleryReviewState::Removed
    } else if group
        .iter()
        .any(|item| item.review_state == GalleryReviewState::Hidden)
    {
        GalleryReviewState::Hidden
    } else if group
        .iter()
        .any(|item| item.review_state == GalleryReviewState::New)
    {
        GalleryReviewState::New
    } else {
        GalleryReviewState::Reviewed
    };

    Ok(GalleryItem {
        catalogue_id: stable,
        title: metadata.title.clone(),
        source_label: metadata.source_label.clone(),
        source_kind: metadata.source_kind,
        media_kind: GalleryMediaKind::Image,
        review_state,
        discovered_at_epoch_seconds: group
            .iter()
            .map(|item| item.discovered_at_epoch_seconds)
            .min()
            .expect("non-empty group"),
        width: metadata.width,
        height: metadata.height,
        video: None,
        thumbnail,
        preview,
    })
}

fn ready_delivery_path(
    availability: GalleryObjectAvailability,
    catalogue_id: &str,
    role: &str,
) -> Option<String> {
    (availability == GalleryObjectAvailability::Ready)
        .then(|| format!("/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gallery_catalogue::{GalleryRepresentation, GalleryRepresentationKind},
        viewed_media::{AdapterKind, CAPTURE_PLAN_SCHEMA_VERSION, CapturePlan, CapturePlanState},
    };

    fn representation(object: &str, length: u64) -> GalleryRepresentation {
        GalleryRepresentation {
            kind: GalleryRepresentationKind::Thumbnail,
            availability: GalleryObjectAvailability::Ready,
            endpoint_id: "endpoint".into(),
            object_store_id: "store".into(),
            object_key: object.into(),
            object_version: 1,
            checksum: format!("sha256:{:0<64}", object),
            content_type: "image/jpeg".into(),
            content_length: length,
            delivery_path: Some("/legacy".into()),
        }
    }

    fn item(id: &str, thumbnail: &str, preview: Option<(&str, u64)>) -> GalleryItem {
        GalleryItem {
            catalogue_id: id.into(),
            title: "Captured image".into(),
            source_label: "X / @fixture".into(),
            source_kind: GallerySourceKind::XAccount,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 42,
            width: 320,
            height: 200,
            video: None,
            thumbnail: representation(thumbnail, 10),
            preview: preview.map(|(object, length)| {
                let mut value = representation(object, length);
                value.kind = GalleryRepresentationKind::OriginalImage;
                value
            }),
        }
    }

    fn plan(id: &str, kind: CaptureKind, settled: bool) -> PendingCapturePlan {
        PendingCapturePlan {
            actor_id: "actor".into(),
            admitted_at_epoch_seconds: 42,
            settled,
            plan: CapturePlan {
                schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
                plan_id: format!("capture-plan-{id}"),
                scheduler_job_id: format!("refresh-{id}"),
                site_id: "x-web".into(),
                origin: "https://x.com".into(),
                canonical_page_url: "https://x.com/home".into(),
                canonical_media_url: "https://pbs.twimg.com/media/fixture?format=jpg&name=small"
                    .into(),
                retrieval_media_url: "https://pbs.twimg.com/media/fixture?format=jpg&name=small"
                    .into(),
                destination: None,
                canonical_presentation_url: format!("https://x.com/fixture/status/{id}"),
                catalogue_id: id.into(),
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                capture_kind: kind,
                width: 320,
                height: 200,
                state: CapturePlanState::AwaitingApprovedAcquisition,
            },
        }
    }

    #[test]
    fn merges_historic_cards_and_keeps_best_original_without_deleting_objects() {
        let result = reconcile_x_image_catalogue(
            vec![
                item("legacy-a", "thumb-a", None),
                item("legacy-b", "thumb-b", Some(("small-original", 20))),
                item("legacy-c", "thumb-c", Some(("large-original", 40))),
            ],
            vec![
                plan("legacy-a", CaptureKind::ObservedThumbnail, true),
                plan("legacy-b", CaptureKind::ExplicitOriginal, true),
                plan("legacy-c", CaptureKind::ExplicitOriginal, true),
            ],
        )
        .unwrap();
        let stable =
            x_image_catalogue_id("https://pbs.twimg.com/media/fixture?format=jpg&name=orig")
                .unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].catalogue_id, stable);
        assert_eq!(result.items[0].thumbnail.object_key, "thumb-a");
        assert_eq!(
            result.items[0].preview.as_ref().unwrap().object_key,
            "large-original"
        );
        assert!(
            result.items[0]
                .preview
                .as_ref()
                .unwrap()
                .delivery_path
                .as_deref()
                .unwrap()
                .contains(&stable)
        );
        assert!(
            result
                .plans
                .iter()
                .all(|pending| pending.plan.catalogue_id == stable)
        );
        assert_eq!(
            result.report,
            XImageReconciliationReport {
                duplicate_groups: 1,
                redundant_cards: 2,
                renamed_cards: 3,
                rebound_plans: 3,
            }
        );
    }

    #[test]
    fn rejects_cross_store_merge() {
        let mut second = item("legacy-b", "thumb-b", None);
        second.thumbnail.object_store_id = "other-store".into();
        assert_eq!(
            reconcile_x_image_catalogue(
                vec![item("legacy-a", "thumb-a", None), second],
                vec![
                    plan("legacy-a", CaptureKind::ObservedThumbnail, true),
                    plan("legacy-b", CaptureKind::ObservedThumbnail, true),
                ],
            ),
            Err(XImageReconciliationError::DestinationMismatch)
        );
    }
}
