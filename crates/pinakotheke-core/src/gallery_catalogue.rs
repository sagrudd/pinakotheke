// SPDX-License-Identifier: MPL-2.0
//! Bounded Monas-hosted catalogue projection for the Pinakotheke media gallery.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::host_context::{AuthenticatedHostContext, HostMode, XIMG_ACCESS};

pub const GALLERY_CATALOGUE_SCHEMA: &str = "pinakotheke.gallery-catalogue.v1";
pub const MAX_GALLERY_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalleryMediaKind {
    Image,
    NormalizedVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySourceKind {
    XAccount,
    Website,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalleryReviewState {
    New,
    Reviewed,
    Hidden,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalleryObjectAvailability {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalleryRepresentationKind {
    Thumbnail,
    OriginalImage,
    VideoPoster,
    NormalizedVideo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryRepresentation {
    pub kind: GalleryRepresentationKind,
    pub availability: GalleryObjectAvailability,
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub checksum: String,
    pub content_type: String,
    pub content_length: u64,
    /// Host-local authorized route. Never an origin or source URL.
    pub delivery_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryItem {
    pub catalogue_id: String,
    pub title: String,
    pub source_label: String,
    pub source_kind: GallerySourceKind,
    pub media_kind: GalleryMediaKind,
    pub review_state: GalleryReviewState,
    pub discovered_at_epoch_seconds: u64,
    pub width: u32,
    pub height: u32,
    pub thumbnail: GalleryRepresentation,
    pub preview: Option<GalleryRepresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryPage {
    pub schema_version: &'static str,
    pub items: Vec<GalleryItem>,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GalleryCatalogueError {
    Unauthorized,
    InvalidPageSize,
    InvalidItem(String),
}

#[derive(Debug, Clone, Default)]
pub struct GalleryCatalogue {
    items: Vec<GalleryItem>,
}

impl GalleryCatalogue {
    pub fn new(mut items: Vec<GalleryItem>) -> Result<Self, GalleryCatalogueError> {
        for item in &items {
            validate_item(item)?;
        }
        items.sort_by(|left, right| {
            right
                .discovered_at_epoch_seconds
                .cmp(&left.discovered_at_epoch_seconds)
                .then_with(|| left.catalogue_id.cmp(&right.catalogue_id))
        });
        Ok(Self { items })
    }

    pub fn page(
        &self,
        context: &AuthenticatedHostContext,
        offset: usize,
        limit: usize,
    ) -> Result<GalleryPage, GalleryCatalogueError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryCatalogueError::Unauthorized);
        }
        if limit == 0 || limit > MAX_GALLERY_PAGE_SIZE {
            return Err(GalleryCatalogueError::InvalidPageSize);
        }
        let items = self
            .items
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_offset = (offset + items.len() < self.items.len()).then_some(offset + items.len());
        Ok(GalleryPage {
            schema_version: GALLERY_CATALOGUE_SCHEMA,
            items,
            next_offset,
        })
    }
}

fn validate_item(item: &GalleryItem) -> Result<(), GalleryCatalogueError> {
    if item.catalogue_id.is_empty() || item.title.is_empty() || item.width == 0 || item.height == 0
    {
        return Err(GalleryCatalogueError::InvalidItem(
            "identity, title, and dimensions are required".into(),
        ));
    }
    validate_representation(&item.thumbnail)?;
    if !matches!(
        item.thumbnail.kind,
        GalleryRepresentationKind::Thumbnail | GalleryRepresentationKind::VideoPoster
    ) {
        return Err(GalleryCatalogueError::InvalidItem(
            "card representation must be a thumbnail or video poster".into(),
        ));
    }
    if let Some(preview) = &item.preview {
        validate_representation(preview)?;
        let expected = match item.media_kind {
            GalleryMediaKind::Image => GalleryRepresentationKind::OriginalImage,
            GalleryMediaKind::NormalizedVideo => GalleryRepresentationKind::NormalizedVideo,
        };
        if preview.kind != expected {
            return Err(GalleryCatalogueError::InvalidItem(
                "preview representation does not match media kind".into(),
            ));
        }
    }
    Ok(())
}

fn validate_representation(
    representation: &GalleryRepresentation,
) -> Result<(), GalleryCatalogueError> {
    if representation.endpoint_id.is_empty()
        || representation.object_store_id.is_empty()
        || representation.object_key.is_empty()
        || !representation.checksum.starts_with("sha256:")
        || representation.content_type.is_empty()
        || representation.content_length == 0
    {
        return Err(GalleryCatalogueError::InvalidItem(
            "representation requires a complete verified ObjectStore reference".into(),
        ));
    }
    match (representation.availability, &representation.delivery_path) {
        (GalleryObjectAvailability::Ready, Some(path))
            if path.starts_with('/') && !path.starts_with("//") =>
        {
            Ok(())
        }
        (GalleryObjectAvailability::Unavailable, None) => Ok(()),
        _ => Err(GalleryCatalogueError::InvalidItem(
            "ready objects require a local delivery path; unavailable objects forbid one".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_context::{HostContextAdapter, MonasHostContextAdapter};

    fn representation(availability: GalleryObjectAvailability) -> GalleryRepresentation {
        GalleryRepresentation {
            kind: GalleryRepresentationKind::Thumbnail,
            availability,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "objects/thumbnail-1".into(),
            checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            content_type: "image/jpeg".into(),
            content_length: 12,
            delivery_path: (availability == GalleryObjectAvailability::Ready)
                .then(|| "/api/gallery/v1/objects/thumbnail-1".into()),
        }
    }

    fn item(id: &str, discovered: u64) -> GalleryItem {
        GalleryItem {
            catalogue_id: id.into(),
            title: "Synthetic redistributable image".into(),
            source_label: "Example website".into(),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: discovered,
            width: 320,
            height: 200,
            thumbnail: representation(GalleryObjectAvailability::Ready),
            preview: None,
        }
    }

    #[test]
    fn returns_a_bounded_newest_first_monas_page() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let catalogue = GalleryCatalogue::new(vec![item("older", 1), item("newer", 2)]).unwrap();
        let page = catalogue.page(&context, 0, 1).unwrap();
        assert_eq!(page.items[0].catalogue_id, "newer");
        assert_eq!(page.next_offset, Some(1));
    }

    #[test]
    fn forbids_origin_fallback_and_inconsistent_availability() {
        let mut invalid = item("invalid", 1);
        invalid.thumbnail.delivery_path = Some("https://example.invalid/image.jpg".into());
        assert!(matches!(
            GalleryCatalogue::new(vec![invalid]),
            Err(GalleryCatalogueError::InvalidItem(_))
        ));

        let mut unavailable = item("unavailable", 1);
        unavailable.thumbnail = representation(GalleryObjectAvailability::Unavailable);
        assert!(GalleryCatalogue::new(vec![unavailable]).is_ok());
    }
}
