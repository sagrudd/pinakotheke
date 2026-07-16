// SPDX-License-Identifier: MPL-2.0
//! Bounded Monas-hosted catalogue projection for the Pinakotheke media gallery.

#![allow(missing_docs)]

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

use crate::{
    host_context::{AuthenticatedHostContext, HostMode, XIMG_ACCESS},
    object_read::AuthorizedObjectReference,
};

pub const GALLERY_CATALOGUE_SCHEMA: &str = "pinakotheke.gallery-catalogue.v1";
pub const MAX_GALLERY_PAGE_SIZE: usize = 200;
pub const GALLERY_STORE_SCHEMA: &str = "pinakotheke.gallery-store.v1";
const MAX_GALLERY_ITEMS: usize = 100_000;
const MAX_GALLERY_FILE_BYTES: u64 = 64 * 1024 * 1024;
static STORE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryImageRole {
    Thumbnail,
    Original,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryImageGrant {
    pub object: AuthorizedObjectReference,
    pub content_type: String,
    pub content_length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryImageResolveError {
    Unauthorized,
    NotFound,
    Unavailable,
    NotAnImage,
}

#[derive(Debug, Clone, Default)]
pub struct GalleryCatalogue {
    items: Vec<GalleryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryStoreDocument {
    schema_version: String,
    items: Vec<GalleryItem>,
}

#[derive(Debug)]
pub enum GalleryStoreError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedSchema,
    TooLarge,
    InvalidCatalogue(GalleryCatalogueError),
}

impl std::fmt::Display for GalleryStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "gallery metadata I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "gallery metadata JSON is invalid: {error}"),
            Self::UnsupportedSchema => {
                formatter.write_str("gallery metadata schema is unsupported")
            }
            Self::TooLarge => formatter.write_str("gallery metadata exceeds its bounded size"),
            Self::InvalidCatalogue(error) => {
                write!(formatter, "gallery metadata is invalid: {error:?}")
            }
        }
    }
}

impl std::error::Error for GalleryStoreError {}

impl From<io::Error> for GalleryStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for GalleryStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryCatalogueStore {
    path: PathBuf,
}

impl GalleryCatalogueStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_empty(&self) -> Result<GalleryCatalogue, GalleryStoreError> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(GalleryCatalogue::default());
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "gallery metadata must be a regular file",
            )
            .into());
        }
        if metadata.len() > MAX_GALLERY_FILE_BYTES {
            return Err(GalleryStoreError::TooLarge);
        }
        let document: GalleryStoreDocument = serde_json::from_slice(&fs::read(&self.path)?)?;
        if document.schema_version != GALLERY_STORE_SCHEMA {
            return Err(GalleryStoreError::UnsupportedSchema);
        }
        if document.items.len() > MAX_GALLERY_ITEMS {
            return Err(GalleryStoreError::TooLarge);
        }
        GalleryCatalogue::new(document.items).map_err(GalleryStoreError::InvalidCatalogue)
    }

    pub fn replace(&self, items: Vec<GalleryItem>) -> Result<(), GalleryStoreError> {
        if items.len() > MAX_GALLERY_ITEMS {
            return Err(GalleryStoreError::TooLarge);
        }
        GalleryCatalogue::new(items.clone()).map_err(GalleryStoreError::InvalidCatalogue)?;
        let document = GalleryStoreDocument {
            schema_version: GALLERY_STORE_SCHEMA.into(),
            items,
        };
        let mut bytes = serde_json::to_vec_pretty(&document)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_GALLERY_FILE_BYTES {
            return Err(GalleryStoreError::TooLarge);
        }
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "gallery path requires a parent",
            )
        })?;
        fs::create_dir_all(parent)?;
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "gallery metadata target must be a regular file",
            )
            .into());
        }
        let temporary = parent.join(format!(
            ".gallery-catalogue.{}.{}.tmp",
            std::process::id(),
            STORE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> io::Result<()> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(Into::into)
    }
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

    pub fn resolve_image(
        &self,
        context: &AuthenticatedHostContext,
        catalogue_id: &str,
        role: GalleryImageRole,
    ) -> Result<GalleryImageGrant, GalleryImageResolveError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryImageResolveError::Unauthorized);
        }
        let item = self
            .items
            .iter()
            .find(|item| item.catalogue_id == catalogue_id)
            .ok_or(GalleryImageResolveError::NotFound)?;
        if item.media_kind != GalleryMediaKind::Image {
            return Err(GalleryImageResolveError::NotAnImage);
        }
        let representation = match role {
            GalleryImageRole::Thumbnail => &item.thumbnail,
            GalleryImageRole::Original => item
                .preview
                .as_ref()
                .ok_or(GalleryImageResolveError::Unavailable)?,
        };
        let expected_kind = match role {
            GalleryImageRole::Thumbnail => GalleryRepresentationKind::Thumbnail,
            GalleryImageRole::Original => GalleryRepresentationKind::OriginalImage,
        };
        if representation.kind != expected_kind
            || !representation.content_type.starts_with("image/")
        {
            return Err(GalleryImageResolveError::NotAnImage);
        }
        if representation.availability != GalleryObjectAvailability::Ready {
            return Err(GalleryImageResolveError::Unavailable);
        }
        Ok(GalleryImageGrant {
            object: AuthorizedObjectReference {
                endpoint_id: representation.endpoint_id.clone(),
                object_store_id: representation.object_store_id.clone(),
                object_key: representation.object_key.clone(),
                checksum: representation.checksum.clone(),
            },
            content_type: representation.content_type.clone(),
            content_length: representation.content_length,
        })
    }

    #[must_use]
    pub fn items(&self) -> &[GalleryItem] {
        &self.items
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

    fn temporary_store() -> (PathBuf, GalleryCatalogueStore) {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-gallery-store-{}-{}",
            std::process::id(),
            STORE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let store = GalleryCatalogueStore::new(root.join("state/gallery-catalogue.v1.json"));
        (root, store)
    }

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

    #[test]
    fn metadata_survives_restart_without_storing_payload_bytes() {
        let (root, store) = temporary_store();
        assert!(store.load_or_empty().unwrap().items.is_empty());
        store.replace(vec![item("persistent-media", 7)]).unwrap();

        let restarted = GalleryCatalogueStore::new(store.path())
            .load_or_empty()
            .unwrap();
        assert_eq!(restarted.items.len(), 1);
        assert_eq!(restarted.items[0].catalogue_id, "persistent-media");
        let bytes = fs::read(store.path()).unwrap();
        assert!(
            !bytes
                .windows(4)
                .any(|window| window == [0xff, 0xd8, 0xff, 0xe0])
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(store.path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_future_and_symlinked_metadata_fail_closed() {
        let (root, store) = temporary_store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), b"{not-json}").unwrap();
        assert!(matches!(
            store.load_or_empty(),
            Err(GalleryStoreError::Json(_))
        ));
        fs::write(
            store.path(),
            b"{\"schema_version\":\"pinakotheke.gallery-store.v2\",\"items\":[]}",
        )
        .unwrap();
        assert!(matches!(
            store.load_or_empty(),
            Err(GalleryStoreError::UnsupportedSchema)
        ));
        #[cfg(unix)]
        {
            fs::remove_file(store.path()).unwrap();
            std::os::unix::fs::symlink(root.join("elsewhere.json"), store.path()).unwrap();
            assert!(matches!(
                store.load_or_empty(),
                Err(GalleryStoreError::Io(_))
            ));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_only_ready_image_roles_for_the_monas_actor() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let catalogue = GalleryCatalogue::new(vec![item("image-1", 1)]).unwrap();
        let grant = catalogue
            .resolve_image(&context, "image-1", GalleryImageRole::Thumbnail)
            .unwrap();
        assert_eq!(grant.object.object_key, "objects/thumbnail-1");
        assert_eq!(grant.content_type, "image/jpeg");
        assert_eq!(
            catalogue.resolve_image(&context, "image-1", GalleryImageRole::Original),
            Err(GalleryImageResolveError::Unavailable)
        );
        assert_eq!(
            catalogue.resolve_image(&context, "missing", GalleryImageRole::Thumbnail),
            Err(GalleryImageResolveError::NotFound)
        );
    }
}
