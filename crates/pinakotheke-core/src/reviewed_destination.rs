// SPDX-License-Identifier: MPL-2.0
//! Actor-scoped authority for an explicitly reviewed DASObjectStore destination.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const REVIEWED_DESTINATION_SCHEMA: &str = "pinakotheke.reviewed-destination.v1";
const STORE_SCHEMA: &str = "pinakotheke.reviewed-destination-store.v1";
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_ACTORS: usize = 256;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedDestinationSelection {
    pub schema_version: String,
    pub revision: u64,
    pub endpoint_id: String,
    pub object_store_id: String,
}

/// Stable identifiers read from the current private capture authority during
/// the one-time migration. This is not a fallback destination: it is consumed
/// only when the actor has no persisted selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritySelectionSeed {
    pub endpoint_id: String,
    pub object_store_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceReviewedDestination {
    pub schema_version: String,
    pub expected_revision: u64,
    pub endpoint_id: String,
    pub object_store_id: String,
}

#[derive(Debug)]
pub enum ReviewedDestinationError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedSchema,
    Invalid,
    TooLarge,
    NotSelected,
    Conflict(ReviewedDestinationSelection),
}

impl std::fmt::Display for ReviewedDestinationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "reviewed destination rejected: {self:?}")
    }
}

impl std::error::Error for ReviewedDestinationError {}

impl From<io::Error> for ReviewedDestinationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ReviewedDestinationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreDocument {
    schema_version: String,
    actors: BTreeMap<String, ReviewedDestinationSelection>,
}

#[derive(Debug, Clone)]
pub struct ReviewedDestinationStore {
    path: PathBuf,
}

impl ReviewedDestinationStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns only the exact actor's persisted selection. No authority seed,
    /// another actor, or first available ObjectStore is used as a fallback.
    pub fn get(
        &self,
        actor_id: &str,
    ) -> Result<ReviewedDestinationSelection, ReviewedDestinationError> {
        validate_actor(actor_id)?;
        self.load()?
            .actors
            .remove(actor_id)
            .ok_or(ReviewedDestinationError::NotSelected)
    }

    /// Seeds an actor once from the already-reviewed capture authority.
    /// Repeated startup is idempotent and never changes an existing selection.
    pub fn seed_from_authority_if_absent(
        &self,
        actor_id: &str,
        seed: &AuthoritySelectionSeed,
    ) -> Result<ReviewedDestinationSelection, ReviewedDestinationError> {
        validate_actor(actor_id)?;
        validate_identifier(&seed.endpoint_id)?;
        validate_identifier(&seed.object_store_id)?;
        let mut document = self.load()?;
        if let Some(existing) = document.actors.get(actor_id) {
            return Ok(existing.clone());
        }
        if document.actors.len() >= MAX_ACTORS {
            return Err(ReviewedDestinationError::TooLarge);
        }
        let selection = ReviewedDestinationSelection {
            schema_version: REVIEWED_DESTINATION_SCHEMA.into(),
            revision: 1,
            endpoint_id: seed.endpoint_id.clone(),
            object_store_id: seed.object_store_id.clone(),
        };
        document.actors.insert(actor_id.into(), selection.clone());
        self.save(&document)?;
        Ok(selection)
    }

    pub fn replace(
        &self,
        actor_id: &str,
        request: ReplaceReviewedDestination,
    ) -> Result<ReviewedDestinationSelection, ReviewedDestinationError> {
        validate_actor(actor_id)?;
        if request.schema_version != REVIEWED_DESTINATION_SCHEMA {
            return Err(ReviewedDestinationError::UnsupportedSchema);
        }
        validate_identifier(&request.endpoint_id)?;
        validate_identifier(&request.object_store_id)?;
        let mut document = self.load()?;
        let current = document.actors.get(actor_id).cloned();
        let current_revision = current.as_ref().map_or(0, |value| value.revision);
        if current_revision != request.expected_revision {
            return match current {
                Some(selection) => Err(ReviewedDestinationError::Conflict(selection)),
                None => Err(ReviewedDestinationError::NotSelected),
            };
        }
        if current.is_none() && document.actors.len() >= MAX_ACTORS {
            return Err(ReviewedDestinationError::TooLarge);
        }
        let selection = ReviewedDestinationSelection {
            schema_version: REVIEWED_DESTINATION_SCHEMA.into(),
            revision: current_revision
                .checked_add(1)
                .ok_or(ReviewedDestinationError::Invalid)?,
            endpoint_id: request.endpoint_id,
            object_store_id: request.object_store_id,
        };
        document.actors.insert(actor_id.into(), selection.clone());
        self.save(&document)?;
        Ok(selection)
    }

    fn load(&self) -> Result<StoreDocument, ReviewedDestinationError> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(empty_document()),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ReviewedDestinationError::Invalid);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(ReviewedDestinationError::Invalid);
            }
        }
        if metadata.len() > MAX_BYTES {
            return Err(ReviewedDestinationError::TooLarge);
        }
        let document: StoreDocument = serde_json::from_slice(&fs::read(&self.path)?)?;
        validate_document(&document)?;
        Ok(document)
    }

    fn save(&self, document: &StoreDocument) -> Result<(), ReviewedDestinationError> {
        validate_document(document)?;
        let mut bytes = serde_json::to_vec_pretty(document)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_BYTES {
            return Err(ReviewedDestinationError::TooLarge);
        }
        let parent = self
            .path
            .parent()
            .ok_or(ReviewedDestinationError::Invalid)?;
        fs::create_dir_all(parent)?;
        if fs::symlink_metadata(parent)?.file_type().is_symlink() {
            return Err(ReviewedDestinationError::Invalid);
        }
        let name = self
            .path
            .file_name()
            .ok_or(ReviewedDestinationError::Invalid)?
            .to_string_lossy();
        let temporary = parent.join(format!(
            ".{name}.{}.{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> Result<(), ReviewedDestinationError> {
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
            drop(file);
            fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

fn empty_document() -> StoreDocument {
    StoreDocument {
        schema_version: STORE_SCHEMA.into(),
        actors: BTreeMap::new(),
    }
}

fn validate_document(document: &StoreDocument) -> Result<(), ReviewedDestinationError> {
    if document.schema_version != STORE_SCHEMA {
        return Err(ReviewedDestinationError::UnsupportedSchema);
    }
    if document.actors.len() > MAX_ACTORS {
        return Err(ReviewedDestinationError::TooLarge);
    }
    for (actor, selection) in &document.actors {
        validate_actor(actor)?;
        if selection.schema_version != REVIEWED_DESTINATION_SCHEMA {
            return Err(ReviewedDestinationError::UnsupportedSchema);
        }
        if selection.revision == 0 {
            return Err(ReviewedDestinationError::Invalid);
        }
        validate_identifier(&selection.endpoint_id)?;
        validate_identifier(&selection.object_store_id)?;
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), ReviewedDestinationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        Err(ReviewedDestinationError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_actor(value: &str) -> Result<(), ReviewedDestinationError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-' | b'@')
        })
    {
        Err(ReviewedDestinationError::Invalid)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pinakotheke-reviewed-destination-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn seed(endpoint: &str, store: &str) -> AuthoritySelectionSeed {
        AuthoritySelectionSeed {
            endpoint_id: endpoint.into(),
            object_store_id: store.into(),
        }
    }

    #[test]
    fn migration_seed_persists_across_restart_and_never_reseeds() {
        let root = root("restart");
        let path = root.join("selection.json");
        let first = ReviewedDestinationStore::new(&path);
        let migrated = first
            .seed_from_authority_if_absent("actor-1", &seed("endpoint-1", "store-1"))
            .unwrap();
        assert_eq!(migrated.revision, 1);
        let restarted = ReviewedDestinationStore::new(&path);
        assert_eq!(restarted.get("actor-1").unwrap(), migrated);
        assert_eq!(
            restarted
                .seed_from_authority_if_absent("actor-1", &seed("other-endpoint", "other-store"))
                .unwrap(),
            migrated
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selection_is_actor_scoped_and_has_no_fallback() {
        let root = root("actor");
        let store = ReviewedDestinationStore::new(root.join("selection.json"));
        store
            .seed_from_authority_if_absent("actor-1", &seed("endpoint-1", "store-1"))
            .unwrap();
        assert!(matches!(
            store.get("actor-2"),
            Err(ReviewedDestinationError::NotSelected)
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn optimistic_revision_rejects_stale_or_future_schema_writes() {
        let root = root("revision");
        let store = ReviewedDestinationStore::new(root.join("selection.json"));
        store
            .seed_from_authority_if_absent("actor-1", &seed("endpoint-1", "store-1"))
            .unwrap();
        let replacement = ReplaceReviewedDestination {
            schema_version: REVIEWED_DESTINATION_SCHEMA.into(),
            expected_revision: 1,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-2".into(),
        };
        let saved = store.replace("actor-1", replacement.clone()).unwrap();
        assert_eq!(saved.revision, 2);
        assert!(matches!(
            store.replace("actor-1", replacement),
            Err(ReviewedDestinationError::Conflict(current)) if current == saved
        ));
        assert!(matches!(
            store.replace(
                "actor-1",
                ReplaceReviewedDestination {
                    schema_version: "pinakotheke.reviewed-destination.v2".into(),
                    expected_revision: 2,
                    endpoint_id: "endpoint-1".into(),
                    object_store_id: "store-3".into(),
                }
            ),
            Err(ReviewedDestinationError::UnsupportedSchema)
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corruption_unknown_fields_and_permissive_files_fail_closed() {
        let root = root("corrupt");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("selection.json");
        fs::write(
            &path,
            r#"{"schema_version":"pinakotheke.reviewed-destination-store.v1","actors":{},"future":true}"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let store = ReviewedDestinationStore::new(&path);
        assert!(matches!(
            store.get("actor-1"),
            Err(ReviewedDestinationError::Json(_))
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(
                &path,
                r#"{"schema_version":"pinakotheke.reviewed-destination-store.v1","actors":{}}"#,
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(matches!(
                store.get("actor-1"),
                Err(ReviewedDestinationError::Invalid)
            ));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_store_is_rejected() {
        use std::os::unix::fs::symlink;
        let root = root("symlink");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.json");
        fs::write(&target, b"{}").unwrap();
        let path = root.join("selection.json");
        symlink(&target, &path).unwrap();
        assert!(matches!(
            ReviewedDestinationStore::new(path).get("actor-1"),
            Err(ReviewedDestinationError::Invalid)
        ));
        let _ = fs::remove_dir_all(root);
    }
}
