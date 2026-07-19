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
pub const GALLERY_FOLDERS_SCHEMA: &str = "pinakotheke.gallery-folders.v1";
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
    #[serde(default = "default_object_version")]
    pub object_version: u64,
    pub checksum: String,
    pub content_type: String,
    pub content_length: u64,
    /// Host-local authorized route. Never an origin or source URL.
    pub delivery_path: Option<String>,
}

const fn default_object_version() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryItem {
    pub catalogue_id: String,
    pub title: String,
    pub source_label: String,
    /// Safe HTTPS presentation page the user can explicitly reopen.
    /// Historic catalogues omit it and remain readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_page_url: Option<String>,
    pub source_kind: GallerySourceKind,
    pub media_kind: GalleryMediaKind,
    pub review_state: GalleryReviewState,
    pub discovered_at_epoch_seconds: u64,
    pub width: u32,
    pub height: u32,
    /// Present on newly admitted normalized videos. Absent only on legacy records.
    #[serde(default)]
    pub video: Option<GalleryVideoMetadata>,
    pub thumbnail: GalleryRepresentation,
    pub preview: Option<GalleryRepresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryVideoMetadata {
    pub duration_millis: u64,
    pub video_codec: String,
    pub audio_codec: String,
    pub profile_id: String,
    pub normalization_state: String,
    pub firefox_playback_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryPage {
    pub schema_version: &'static str,
    pub items: Vec<GalleryItem>,
    pub next_offset: Option<usize>,
    pub matched_items: usize,
    pub total_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryFolderPage {
    pub schema_version: &'static str,
    pub prefix: String,
    pub breadcrumbs: Vec<GalleryFolderBreadcrumb>,
    pub folders: Vec<GalleryFolderEntry>,
    pub matched_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryFolderBreadcrumb {
    pub name: String,
    pub prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalleryFolderEntry {
    pub name: String,
    pub prefix: String,
    pub item_count: usize,
    pub latest_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GalleryCatalogueFilter {
    pub source_kind: Option<GallerySourceKind>,
    pub media_kind: Option<GalleryMediaKind>,
    pub review_state: Option<GalleryReviewState>,
    pub availability: Option<GalleryObjectAvailability>,
    pub discovered_from_epoch_seconds: Option<u64>,
    pub discovered_to_epoch_seconds: Option<u64>,
    pub text: Option<String>,
    pub object_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GalleryCatalogueError {
    Unauthorized,
    InvalidPageSize,
    InvalidFilter,
    InvalidItem(String),
    NotFound,
    DeleteConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryDeletePlan {
    pub catalogue_id: String,
    pub affected_catalogue_ids: Vec<String>,
    pub objects: Vec<AuthorizedObjectReference>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryVideoGrant {
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
        self.filtered_page(context, offset, limit, &GalleryCatalogueFilter::default())
    }

    pub fn filtered_page(
        &self,
        context: &AuthenticatedHostContext,
        offset: usize,
        limit: usize,
        filter: &GalleryCatalogueFilter,
    ) -> Result<GalleryPage, GalleryCatalogueError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryCatalogueError::Unauthorized);
        }
        if limit == 0 || limit > MAX_GALLERY_PAGE_SIZE {
            return Err(GalleryCatalogueError::InvalidPageSize);
        }
        let normalized_text = validate_filter(filter)?;
        let object_prefix = validate_object_prefix(filter.object_prefix.as_deref())?;
        let matches = self.items.iter().filter(|item| {
            filter
                .source_kind
                .is_none_or(|value| item.source_kind == value)
                && filter
                    .media_kind
                    .is_none_or(|value| item.media_kind == value)
                && filter
                    .review_state
                    .is_none_or(|value| item.review_state == value)
                && filter.availability.is_none_or(|value| {
                    item.thumbnail.availability == value
                        || item
                            .preview
                            .as_ref()
                            .is_some_and(|preview| preview.availability == value)
                })
                && filter
                    .discovered_from_epoch_seconds
                    .is_none_or(|value| item.discovered_at_epoch_seconds >= value)
                && filter
                    .discovered_to_epoch_seconds
                    .is_none_or(|value| item.discovered_at_epoch_seconds <= value)
                && normalized_text.as_ref().is_none_or(|text| {
                    item.title.to_lowercase().contains(text)
                        || item.source_label.to_lowercase().contains(text)
                        || item.catalogue_id.to_lowercase().contains(text)
                })
                && object_prefix.as_ref().is_none_or(|prefix| {
                    key_in_prefix(&item.thumbnail.object_key, prefix)
                        || item
                            .preview
                            .as_ref()
                            .is_some_and(|preview| key_in_prefix(&preview.object_key, prefix))
                })
        });
        let matched_items = matches.clone().count();
        let items = matches
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_offset = (offset + items.len() < matched_items).then_some(offset + items.len());
        Ok(GalleryPage {
            schema_version: GALLERY_CATALOGUE_SCHEMA,
            items,
            next_offset,
            matched_items,
            total_items: self.items.len(),
        })
    }

    pub fn folder_page(
        &self,
        context: &AuthenticatedHostContext,
        prefix: Option<&str>,
    ) -> Result<GalleryFolderPage, GalleryCatalogueError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryCatalogueError::Unauthorized);
        }
        let prefix = validate_object_prefix(prefix)?.unwrap_or_default();
        let mut children = std::collections::BTreeMap::<String, (usize, u64)>::new();
        let mut matched_items = 0usize;
        for item in &self.items {
            let key = &item.thumbnail.object_key;
            if !prefix.is_empty() && !key_in_prefix(key, &prefix) {
                continue;
            }
            matched_items += 1;
            let remainder = key.strip_prefix(&prefix).unwrap_or(key);
            let remainder = remainder.strip_prefix('/').unwrap_or(remainder);
            let Some(name) = remainder.split('/').next().filter(|name| !name.is_empty()) else {
                continue;
            };
            if !remainder.contains('/') {
                continue;
            }
            let child_prefix = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            let entry = children.entry(child_prefix).or_default();
            entry.0 += 1;
            entry.1 = entry.1.max(item.discovered_at_epoch_seconds);
        }
        let breadcrumbs = prefix
            .split('/')
            .filter(|part| !part.is_empty())
            .scan(String::new(), |path, name| {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(name);
                Some(GalleryFolderBreadcrumb {
                    name: name.to_owned(),
                    prefix: path.clone(),
                })
            })
            .collect();
        Ok(GalleryFolderPage {
            schema_version: GALLERY_FOLDERS_SCHEMA,
            prefix,
            breadcrumbs,
            folders: children
                .into_iter()
                .map(
                    |(prefix, (item_count, latest_at_epoch_seconds))| GalleryFolderEntry {
                        name: prefix.rsplit('/').next().unwrap_or(&prefix).to_owned(),
                        prefix,
                        item_count,
                        latest_at_epoch_seconds,
                    },
                )
                .collect(),
            matched_items,
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
        let representation = match role {
            GalleryImageRole::Thumbnail => &item.thumbnail,
            GalleryImageRole::Original => item
                .preview
                .as_ref()
                .ok_or(GalleryImageResolveError::Unavailable)?,
        };
        let expected_kind = match (item.media_kind, role) {
            (GalleryMediaKind::Image, GalleryImageRole::Thumbnail) => {
                GalleryRepresentationKind::Thumbnail
            }
            (GalleryMediaKind::Image, GalleryImageRole::Original) => {
                GalleryRepresentationKind::OriginalImage
            }
            (GalleryMediaKind::NormalizedVideo, GalleryImageRole::Thumbnail) => {
                GalleryRepresentationKind::VideoPoster
            }
            (GalleryMediaKind::NormalizedVideo, GalleryImageRole::Original) => {
                return Err(GalleryImageResolveError::NotAnImage);
            }
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
                object_version: representation.object_version,
                checksum: representation.checksum.clone(),
            },
            content_type: representation.content_type.clone(),
            content_length: representation.content_length,
        })
    }

    pub fn resolve_video(
        &self,
        context: &AuthenticatedHostContext,
        catalogue_id: &str,
    ) -> Result<GalleryVideoGrant, GalleryImageResolveError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryImageResolveError::Unauthorized);
        }
        let item = self
            .items
            .iter()
            .find(|item| item.catalogue_id == catalogue_id)
            .ok_or(GalleryImageResolveError::NotFound)?;
        if item.media_kind != GalleryMediaKind::NormalizedVideo {
            return Err(GalleryImageResolveError::NotAnImage);
        }
        let representation = item
            .preview
            .as_ref()
            .ok_or(GalleryImageResolveError::Unavailable)?;
        if representation.kind != GalleryRepresentationKind::NormalizedVideo
            || !representation.content_type.starts_with("video/")
        {
            return Err(GalleryImageResolveError::NotAnImage);
        }
        if representation.availability != GalleryObjectAvailability::Ready {
            return Err(GalleryImageResolveError::Unavailable);
        }
        Ok(GalleryVideoGrant {
            object: AuthorizedObjectReference {
                endpoint_id: representation.endpoint_id.clone(),
                object_store_id: representation.object_store_id.clone(),
                object_key: representation.object_key.clone(),
                object_version: representation.object_version,
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

    pub(crate) fn items_mut(&mut self) -> &mut [GalleryItem] {
        &mut self.items
    }

    /// Plans deletion of the selected asset and every duplicate catalogue row
    /// that shares one of its exact immutable object references.
    pub fn delete_plan(
        &self,
        context: &AuthenticatedHostContext,
        catalogue_id: &str,
    ) -> Result<GalleryDeletePlan, GalleryCatalogueError> {
        if context.host_mode() != HostMode::MonasStandalone || !context.permits(XIMG_ACCESS) {
            return Err(GalleryCatalogueError::Unauthorized);
        }
        let selected = self
            .items
            .iter()
            .find(|item| item.catalogue_id == catalogue_id)
            .ok_or(GalleryCatalogueError::NotFound)?;
        let mut objects = representation_objects(selected);
        let mut affected = std::collections::BTreeSet::from([catalogue_id.to_owned()]);
        loop {
            let mut changed = false;
            for item in &self.items {
                let item_objects = representation_objects(item);
                if item_objects.iter().any(|object| objects.contains(object))
                    && affected.insert(item.catalogue_id.clone())
                {
                    objects.extend(item_objects);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(GalleryDeletePlan {
            catalogue_id: catalogue_id.to_owned(),
            affected_catalogue_ids: affected.into_iter().collect(),
            objects: objects.into_iter().collect(),
        })
    }

    /// Removes only the catalogue rows represented by an unchanged reviewed
    /// plan. The caller must first verify every planned object deletion at the
    /// DASObjectStore authority.
    pub fn apply_verified_delete(
        &mut self,
        context: &AuthenticatedHostContext,
        plan: &GalleryDeletePlan,
    ) -> Result<(), GalleryCatalogueError> {
        let current = self.delete_plan(context, &plan.catalogue_id)?;
        if current != *plan {
            return Err(GalleryCatalogueError::DeleteConflict);
        }
        let affected = plan
            .affected_catalogue_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        self.items
            .retain(|item| !affected.contains(&item.catalogue_id));
        Ok(())
    }
}

fn representation_objects(
    item: &GalleryItem,
) -> std::collections::BTreeSet<AuthorizedObjectReference> {
    std::iter::once(&item.thumbnail)
        .chain(item.preview.iter())
        .map(|representation| AuthorizedObjectReference {
            endpoint_id: representation.endpoint_id.clone(),
            object_store_id: representation.object_store_id.clone(),
            object_key: representation.object_key.clone(),
            object_version: representation.object_version,
            checksum: representation.checksum.clone(),
        })
        .collect()
}

fn validate_filter(
    filter: &GalleryCatalogueFilter,
) -> Result<Option<String>, GalleryCatalogueError> {
    if filter
        .discovered_from_epoch_seconds
        .zip(filter.discovered_to_epoch_seconds)
        .is_some_and(|(from, to)| from > to)
    {
        return Err(GalleryCatalogueError::InvalidFilter);
    }
    let Some(text) = filter.text.as_deref() else {
        return Ok(None);
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if text.chars().count() > 128 || text.chars().any(char::is_control) {
        return Err(GalleryCatalogueError::InvalidFilter);
    }
    Ok(Some(text.to_lowercase()))
}

fn validate_object_prefix(prefix: Option<&str>) -> Result<Option<String>, GalleryCatalogueError> {
    let Some(prefix) = prefix.map(str::trim).filter(|prefix| !prefix.is_empty()) else {
        return Ok(None);
    };
    if prefix.len() > 512
        || prefix.starts_with('/')
        || prefix.ends_with('/')
        || prefix.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.len() > 128
                || part
                    .bytes()
                    .any(|byte| byte.is_ascii_control() || byte == b'\\')
        })
    {
        return Err(GalleryCatalogueError::InvalidFilter);
    }
    Ok(Some(prefix.to_owned()))
}

fn key_in_prefix(key: &str, prefix: &str) -> bool {
    key == prefix
        || key
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

fn validate_item(item: &GalleryItem) -> Result<(), GalleryCatalogueError> {
    if item.catalogue_id.is_empty() || item.title.is_empty() || item.width == 0 || item.height == 0
    {
        return Err(GalleryCatalogueError::InvalidItem(
            "identity, title, and dimensions are required".into(),
        ));
    }
    if item.source_page_url.as_deref().is_some_and(|value| {
        value.len() > 2_048
            || !value.starts_with("https://")
            || value.contains(['\n', '\r'])
            || value.parse::<url::Url>().ok().is_none_or(|url| {
                url.scheme() != "https"
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
            })
    }) {
        return Err(GalleryCatalogueError::InvalidItem(
            "source page must be a bounded credential-free HTTPS URL".into(),
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
    match (item.media_kind, &item.video) {
        (GalleryMediaKind::Image, Some(_)) => {
            return Err(GalleryCatalogueError::InvalidItem(
                "image records cannot carry video metadata".into(),
            ));
        }
        (GalleryMediaKind::NormalizedVideo, Some(video))
            if video.duration_millis == 0
                || !short_token(&video.video_codec)
                || !short_token(&video.audio_codec)
                || !short_token(&video.profile_id)
                || video.normalization_state != "ready"
                || !short_token(&video.firefox_playback_evidence_id) =>
        {
            return Err(GalleryCatalogueError::InvalidItem(
                "video metadata is incomplete or not ready".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

fn short_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn validate_representation(
    representation: &GalleryRepresentation,
) -> Result<(), GalleryCatalogueError> {
    if representation.endpoint_id.is_empty()
        || representation.object_store_id.is_empty()
        || representation.object_key.is_empty()
        || representation.object_version == 0
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
            object_version: 1,
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
            source_page_url: Some("https://example.invalid/gallery".into()),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: discovered,
            width: 320,
            height: 200,
            video: None,
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
        assert_eq!(page.matched_items, 2);
        assert_eq!(page.total_items, 2);
    }

    #[test]
    fn folder_pages_browse_immediate_children_and_filter_exact_prefixes() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let mut first = item("first", 10);
        first.thumbnail.object_key = "x.com/artist_one/observed_thumbnail/first".into();
        let mut second = item("second", 20);
        second.thumbnail.object_key = "x.com/artist_two/explicit_original/second".into();
        let mut prefix_collision = item("collision", 30);
        prefix_collision.thumbnail.object_key =
            "x.com/artist_two_extra/observed_thumbnail/collision".into();
        let catalogue = GalleryCatalogue::new(vec![first, second, prefix_collision]).unwrap();

        let root = catalogue.folder_page(&context, None).unwrap();
        assert_eq!(root.folders[0].prefix, "x.com");
        assert_eq!(root.folders[0].item_count, 3);
        let artists = catalogue.folder_page(&context, Some("x.com")).unwrap();
        assert_eq!(artists.folders.len(), 3);
        assert_eq!(artists.breadcrumbs[0].name, "x.com");
        let page = catalogue
            .filtered_page(
                &context,
                0,
                20,
                &GalleryCatalogueFilter {
                    object_prefix: Some("x.com/artist_two".into()),
                    ..GalleryCatalogueFilter::default()
                },
            )
            .unwrap();
        assert_eq!(page.matched_items, 1);
        assert_eq!(page.items[0].catalogue_id, "second");
        assert_eq!(
            catalogue.folder_page(&context, Some("../unsafe")),
            Err(GalleryCatalogueError::InvalidFilter)
        );
    }

    #[test]
    fn object_version_survives_resolution_and_legacy_records_default_to_one() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let mut versioned = item("versioned", 1);
        versioned.thumbnail.object_version = 7;
        let catalogue = GalleryCatalogue::new(vec![versioned]).unwrap();
        let grant = catalogue
            .resolve_image(&context, "versioned", GalleryImageRole::Thumbnail)
            .unwrap();
        assert_eq!(grant.object.object_version, 7);

        let legacy: GalleryRepresentation = serde_json::from_str(
            r#"{"kind":"thumbnail","availability":"ready","endpoint_id":"endpoint-1","object_store_id":"store-1","object_key":"objects/legacy","checksum":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","content_type":"image/jpeg","content_length":12,"delivery_path":"/legacy"}"#,
        )
        .unwrap();
        assert_eq!(legacy.object_version, 1);
    }

    #[test]
    fn filters_before_pagination_and_rejects_unbounded_queries() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let mut video = item("video", 3);
        video.title = "A calm ocean film".into();
        video.media_kind = GalleryMediaKind::NormalizedVideo;
        video.review_state = GalleryReviewState::Reviewed;
        video.thumbnail.kind = GalleryRepresentationKind::VideoPoster;
        video.preview = Some(GalleryRepresentation {
            kind: GalleryRepresentationKind::NormalizedVideo,
            content_type: "video/mp4".into(),
            ..representation(GalleryObjectAvailability::Ready)
        });
        let catalogue = GalleryCatalogue::new(vec![item("image", 2), video]).unwrap();
        let page = catalogue
            .filtered_page(
                &context,
                0,
                1,
                &GalleryCatalogueFilter {
                    media_kind: Some(GalleryMediaKind::NormalizedVideo),
                    review_state: Some(GalleryReviewState::Reviewed),
                    text: Some(" OCEAN ".into()),
                    ..GalleryCatalogueFilter::default()
                },
            )
            .unwrap();
        assert_eq!(page.items[0].catalogue_id, "video");
        assert_eq!(page.matched_items, 1);
        assert_eq!(page.total_items, 2);
        assert_eq!(page.next_offset, None);

        assert_eq!(
            catalogue.filtered_page(
                &context,
                0,
                20,
                &GalleryCatalogueFilter {
                    discovered_from_epoch_seconds: Some(4),
                    discovered_to_epoch_seconds: Some(3),
                    ..GalleryCatalogueFilter::default()
                }
            ),
            Err(GalleryCatalogueError::InvalidFilter)
        );
        assert_eq!(
            catalogue.filtered_page(
                &context,
                0,
                20,
                &GalleryCatalogueFilter {
                    text: Some("x".repeat(129)),
                    ..GalleryCatalogueFilter::default()
                }
            ),
            Err(GalleryCatalogueError::InvalidFilter)
        );
    }

    #[test]
    fn forbids_origin_fallback_and_inconsistent_availability() {
        let mut invalid = item("invalid", 1);
        invalid.thumbnail.delivery_path = Some("https://example.invalid/image.jpg".into());
        assert!(matches!(
            GalleryCatalogue::new(vec![invalid]),
            Err(GalleryCatalogueError::InvalidItem(_))
        ));

        let mut unversioned = item("unversioned", 1);
        unversioned.thumbnail.object_version = 0;
        assert!(matches!(
            GalleryCatalogue::new(vec![unversioned]),
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
    fn verified_delete_removes_shared_duplicate_rows_only_after_exact_plan() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let mut first = item("first", 2);
        first.preview = Some(GalleryRepresentation {
            kind: GalleryRepresentationKind::OriginalImage,
            object_key: "objects/original-1".into(),
            ..representation(GalleryObjectAvailability::Ready)
        });
        let mut duplicate = item("duplicate", 1);
        duplicate.thumbnail = first.thumbnail.clone();
        duplicate.preview = first.preview.clone();
        let mut unrelated = item("unrelated", 3);
        unrelated.thumbnail.object_key = "objects/unrelated".into();
        let mut catalogue = GalleryCatalogue::new(vec![first, duplicate, unrelated]).unwrap();

        let plan = catalogue.delete_plan(&context, "first").unwrap();
        assert_eq!(plan.affected_catalogue_ids, ["duplicate", "first"]);
        assert_eq!(plan.objects.len(), 2);
        assert_eq!(catalogue.items().len(), 3, "planning is non-mutating");

        catalogue.apply_verified_delete(&context, &plan).unwrap();
        assert_eq!(catalogue.items().len(), 1);
        assert_eq!(catalogue.items()[0].catalogue_id, "unrelated");
    }

    #[test]
    fn verified_delete_rejects_a_stale_plan() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let mut catalogue = GalleryCatalogue::new(vec![item("selected", 1)]).unwrap();
        let plan = catalogue.delete_plan(&context, "selected").unwrap();
        catalogue.items[0].thumbnail.object_version = 2;
        assert_eq!(
            catalogue.apply_verified_delete(&context, &plan),
            Err(GalleryCatalogueError::DeleteConflict)
        );
        assert_eq!(catalogue.items().len(), 1);
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
