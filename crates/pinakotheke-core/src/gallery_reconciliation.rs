// SPDX-License-Identifier: MPL-2.0
//! Authority-driven reconciliation of the gallery metadata projection.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::gallery_catalogue::{
    GalleryCatalogue, GalleryObjectAvailability, GalleryRepresentation,
};

pub const GALLERY_CONVERGENCE_SCHEMA: &str = "pinakotheke.gallery-convergence.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityObjectIdentity {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityObject {
    #[serde(flatten)]
    pub identity: AuthorityObjectIdentity,
    /// Immutable DAS representation version. A key alone is never evidence
    /// that the projected representation is still the committed object.
    pub object_version: u64,
    /// Immutable checksum reported by DASObjectStore for this version.
    pub checksum: String,
    pub state: String,
    pub content_length: u64,
}

impl AuthorityObject {
    /// Builds the only authority state that can make a gallery representation
    /// available. Callers which observe another state must retain that state
    /// explicitly; reconciliation deliberately treats it as unavailable.
    #[must_use]
    pub fn protected(
        identity: AuthorityObjectIdentity,
        object_version: u64,
        checksum: impl Into<String>,
        content_length: u64,
    ) -> Self {
        Self {
            identity,
            object_version,
            checksum: checksum.into(),
            state: "Protected".into(),
            content_length,
        }
    }

    fn matches_representation(&self, representation: &GalleryRepresentation) -> bool {
        self.state == "Protected"
            && self.identity == representation_identity(representation)
            && self.object_version == representation.object_version
            && self.checksum == representation.checksum
            && self.content_length == representation.content_length
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryConvergenceReport {
    pub schema_version: &'static str,
    pub authoritative_count: usize,
    pub projected_count: usize,
    pub orphan_count: usize,
    pub stale_count: usize,
    pub changed_representations: usize,
}

impl Default for GalleryConvergenceReport {
    fn default() -> Self {
        Self {
            schema_version: GALLERY_CONVERGENCE_SCHEMA,
            authoritative_count: 0,
            projected_count: 0,
            orphan_count: 0,
            stale_count: 0,
            changed_representations: 0,
        }
    }
}

/// Reconciles availability against a complete, settled authority inventory.
///
/// Every representation must match its protected DAS evidence exactly: endpoint,
/// logical store, object key, immutable version, checksum, and byte length.
/// A key match with changed immutable evidence is a tombstone, not a restored
/// object. DAS object identifiers are immutable within a logical store; the
/// inventory must omit incomplete, deleted, or otherwise non-settled records.
pub fn reconcile_gallery(
    gallery: &mut GalleryCatalogue,
    authority: &[AuthorityObject],
) -> GalleryConvergenceReport {
    let authority = authority
        .iter()
        .filter(|object| object.state == "Protected")
        .fold(
            BTreeMap::<AuthorityObjectIdentity, Vec<&AuthorityObject>>::new(),
            |mut grouped, object| {
                grouped
                    .entry(object.identity.clone())
                    .or_default()
                    .push(object);
                grouped
            },
        );
    let mut referenced = BTreeSet::new();
    let mut available = BTreeSet::new();
    let mut stale = BTreeSet::new();
    let mut changed_representations = 0;

    for item in gallery.items_mut() {
        let catalogue_id = item.catalogue_id.clone();
        for representation in std::iter::once(&mut item.thumbnail).chain(item.preview.iter_mut()) {
            let previous_availability = representation.availability;
            let previous_delivery_path = representation.delivery_path.clone();
            let identity = representation_identity(representation);
            referenced.insert(identity.clone());
            let is_ready = authority.get(&identity).is_some_and(|objects| {
                // A complete inventory has one settled representation for
                // a stable DAS object identity. Multiple records are
                // ambiguous authority evidence, so fail closed rather
                // than accepting an arbitrary matching entry.
                objects.len() == 1 && objects[0].matches_representation(representation)
            });
            if is_ready {
                available.insert(identity);
            } else {
                stale.insert(identity);
            }
            let availability = if is_ready {
                GalleryObjectAvailability::Ready
            } else {
                GalleryObjectAvailability::Unavailable
            };
            representation.availability = availability;
            match availability {
                GalleryObjectAvailability::Ready if representation.delivery_path.is_none() => {
                    let role = match representation.kind {
                        crate::gallery_catalogue::GalleryRepresentationKind::Thumbnail
                        | crate::gallery_catalogue::GalleryRepresentationKind::VideoPoster => {
                            "thumbnail"
                        }
                        crate::gallery_catalogue::GalleryRepresentationKind::OriginalImage => {
                            "original"
                        }
                        crate::gallery_catalogue::GalleryRepresentationKind::NormalizedVideo => {
                            "video"
                        }
                    };
                    representation.delivery_path = Some(format!(
                        "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}"
                    ));
                }
                GalleryObjectAvailability::Unavailable => representation.delivery_path = None,
                _ => {}
            }
            if representation.availability != previous_availability
                || representation.delivery_path != previous_delivery_path
            {
                changed_representations += 1;
            }
        }
    }

    GalleryConvergenceReport {
        schema_version: GALLERY_CONVERGENCE_SCHEMA,
        authoritative_count: authority.len(),
        projected_count: available.len(),
        orphan_count: authority
            .keys()
            .filter(|identity| !referenced.contains(*identity))
            .count(),
        stale_count: stale.len(),
        changed_representations,
    }
}

fn representation_identity(representation: &GalleryRepresentation) -> AuthorityObjectIdentity {
    AuthorityObjectIdentity {
        endpoint_id: representation.endpoint_id.clone(),
        object_store_id: representation.object_store_id.clone(),
        object_key: representation.object_key.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery_catalogue::{
        GalleryItem, GalleryMediaKind, GalleryRepresentationKind, GalleryReviewState,
        GallerySourceKind,
    };

    fn representation(key: &str) -> GalleryRepresentation {
        GalleryRepresentation {
            kind: GalleryRepresentationKind::Thumbnail,
            availability: GalleryObjectAvailability::Ready,
            endpoint_id: "endpoint-a".into(),
            object_store_id: "store-a".into(),
            object_key: key.into(),
            object_version: 1,
            checksum: format!("sha256:{key}"),
            content_type: "image/jpeg".into(),
            content_length: 12,
            delivery_path: Some(format!("/objects/{key}")),
        }
    }

    fn gallery() -> GalleryCatalogue {
        let mut original = representation("original");
        original.kind = GalleryRepresentationKind::OriginalImage;
        GalleryCatalogue::new(vec![GalleryItem {
            catalogue_id: "item-a".into(),
            title: "Item".into(),
            source_label: "Creator".into(),
            source_page_url: None,
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 10,
            height: 10,
            video: None,
            thumbnail: representation("thumb"),
            preview: Some(original),
        }])
        .unwrap()
    }

    fn authority(key: &str) -> AuthorityObject {
        AuthorityObject::protected(
            AuthorityObjectIdentity {
                endpoint_id: "endpoint-a".into(),
                object_store_id: "store-a".into(),
                object_key: key.into(),
            },
            1,
            format!("sha256:{key}"),
            12,
        )
    }

    #[test]
    fn external_deletion_tombstones_only_the_missing_representation() {
        let mut gallery = gallery();
        let report = reconcile_gallery(&mut gallery, &[authority("thumb")]);
        assert_eq!(report.authoritative_count, 1);
        assert_eq!(report.projected_count, 1);
        assert_eq!(report.stale_count, 1);
        assert_eq!(report.changed_representations, 1);
        assert_eq!(
            gallery.items()[0].preview.as_ref().unwrap().availability,
            GalleryObjectAvailability::Unavailable
        );
        assert_eq!(
            gallery.items()[0].thumbnail.availability,
            GalleryObjectAvailability::Ready
        );
    }

    #[test]
    fn returned_authority_restores_availability_and_reports_orphans() {
        let mut gallery = gallery();
        reconcile_gallery(&mut gallery, &[]);
        let report = reconcile_gallery(
            &mut gallery,
            &[
                authority("thumb"),
                authority("original"),
                authority("orphan"),
            ],
        );
        assert_eq!(report.projected_count, 2);
        assert_eq!(report.orphan_count, 1);
        assert_eq!(report.stale_count, 0);
        assert_eq!(report.changed_representations, 2);
    }

    #[test]
    fn duplicate_projection_references_count_once() {
        let mut gallery = gallery();
        let mut duplicate = gallery.items()[0].clone();
        duplicate.catalogue_id = "item-b".into();
        gallery = GalleryCatalogue::new(vec![gallery.items()[0].clone(), duplicate]).unwrap();
        let report = reconcile_gallery(&mut gallery, &[authority("thumb"), authority("original")]);
        assert_eq!(report.authoritative_count, 2);
        assert_eq!(report.projected_count, 2);
        assert_eq!(report.stale_count, 0);
    }

    #[test]
    fn non_protected_authority_state_never_claims_gallery_availability() {
        let mut gallery = gallery();
        let mut staging = authority("thumb");
        staging.state = "HashVerified".into();
        let report = reconcile_gallery(&mut gallery, &[staging]);
        assert_eq!(report.authoritative_count, 0);
        assert_eq!(report.projected_count, 0);
        assert_eq!(report.stale_count, 2);
    }

    #[test]
    fn immutable_authority_mismatches_tombstone_the_representation() {
        let cases = [
            ("version", {
                let mut object = authority("thumb");
                object.object_version = 2;
                object
            }),
            ("checksum", {
                let mut object = authority("thumb");
                object.checksum = "sha256:different".into();
                object
            }),
            ("length", {
                let mut object = authority("thumb");
                object.content_length = 13;
                object
            }),
        ];

        for (name, object) in cases {
            let mut catalogue = gallery();
            let report = reconcile_gallery(&mut catalogue, &[object]);
            assert_eq!(report.authoritative_count, 1, "{name}");
            assert_eq!(report.projected_count, 0, "{name}");
            assert_eq!(report.stale_count, 2, "{name}");
            assert_eq!(report.changed_representations, 2, "{name}");
            assert_eq!(
                catalogue.items()[0].thumbnail.availability,
                GalleryObjectAvailability::Unavailable,
                "{name}"
            );
            assert!(
                catalogue.items()[0].thumbnail.delivery_path.is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn exact_authority_restores_a_representation_after_immutable_mismatch() {
        let mut catalogue = gallery();
        let mut stale_authority = authority("thumb");
        stale_authority.checksum = "sha256:replaced".into();
        reconcile_gallery(&mut catalogue, &[stale_authority]);

        let report = reconcile_gallery(&mut catalogue, &[authority("thumb")]);
        assert_eq!(report.authoritative_count, 1);
        assert_eq!(report.projected_count, 1);
        assert_eq!(report.stale_count, 1);
        assert_eq!(report.changed_representations, 1);
        assert_eq!(
            catalogue.items()[0].thumbnail.availability,
            GalleryObjectAvailability::Ready
        );
        assert_eq!(
            catalogue.items()[0].thumbnail.delivery_path.as_deref(),
            Some("/products/pinakotheke/api/gallery/v1/objects/item-a/thumbnail")
        );
    }

    #[test]
    fn duplicate_authority_evidence_is_ambiguous_and_tombstoned() {
        let mut catalogue = gallery();
        let report = reconcile_gallery(&mut catalogue, &[authority("thumb"), authority("thumb")]);
        assert_eq!(report.authoritative_count, 1);
        assert_eq!(report.projected_count, 0);
        assert_eq!(report.orphan_count, 0);
        assert_eq!(report.stale_count, 2);
        assert_eq!(report.changed_representations, 2);
    }
}
