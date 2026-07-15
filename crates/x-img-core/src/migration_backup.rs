// SPDX-License-Identifier: MPL-2.0
//! Strict metadata export, migration, and restore proof boundary.
//!
//! Snapshots contain configuration and immutable catalogue/pairing references
//! only. Media bytes, credentials, cookies, sessions, and capabilities are not
//! representable by this schema.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x_img_model::{INSTANCE_SCHEMA_VERSION, InstanceConfig};

/// Current metadata snapshot schema.
pub const SNAPSHOT_SCHEMA: &str = "x-img.metadata-snapshot.v1";

/// A strict, portable snapshot of x-img-owned metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataSnapshot {
    /// Snapshot schema identifier.
    pub schema_version: String,
    /// Stable export identifier, supplied by the authenticated host.
    pub export_id: String,
    /// RFC 3339 export time.
    pub exported_at: String,
    /// Complete validated instance configuration.
    pub configuration: InstanceConfig,
    /// Immutable committed catalogue references.
    pub catalogue: Vec<CatalogueSnapshotRecord>,
    /// Non-secret extension pairing identities requiring reviewed re-pairing.
    pub extension_pairings: Vec<PairingSnapshotRecord>,
}

/// Immutable catalogue evidence retained across export and restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueSnapshotRecord {
    /// Stable canonical media identity.
    pub canonical_media_identity: String,
    /// Stable DASObjectStore endpoint identity.
    pub endpoint_id: String,
    /// Stable logical ObjectStore identity.
    pub object_store_id: String,
    /// Stable authority object reference.
    pub object_reference_id: String,
    /// Immutable lowercase SHA-256.
    pub checksum_sha256: String,
    /// Historic adapter/product label, preserved rather than rewritten.
    pub historic_label: String,
}

/// Safe extension pairing identity; no token, CSRF value, or session is stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairingSnapshotRecord {
    /// Stable browser profile identity.
    pub profile_id: String,
    /// Exact paired x-img HTTPS origin.
    pub instance_origin: String,
    /// Restore always requires explicit reviewed re-pairing.
    pub requires_reviewed_repair: bool,
}

/// Canonical bytes and independent checksum presented to backup storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportArtifact {
    /// Pretty JSON bytes; metadata only.
    pub bytes: Vec<u8>,
    /// Lowercase SHA-256 over `bytes`.
    pub checksum_sha256: String,
}

/// Result of a copy-on-write legacy migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationOutput {
    /// Exact verified pre-migration backup.
    pub backup: ExportArtifact,
    /// Migrated snapshot. Historic identities and references remain unchanged.
    pub snapshot: MetadataSnapshot,
    /// Bounded operator-facing result code.
    pub result: &'static str,
}

/// Strict snapshot failure.
#[derive(Debug)]
pub enum SnapshotError {
    /// Snapshot JSON is invalid or contains unknown fields.
    Json(serde_json::Error),
    /// A schema is unknown or from a future major.
    UnsupportedSchema,
    /// Snapshot checksum does not match the reviewed backup.
    ChecksumMismatch,
    /// Snapshot metadata violates a bounded invariant.
    Invalid(&'static str),
    /// Configuration validation failed.
    Configuration(x_img_model::ConfigValidationError),
}

impl From<serde_json::Error> for SnapshotError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<x_img_model::ConfigValidationError> for SnapshotError {
    fn from(value: x_img_model::ConfigValidationError) -> Self {
        Self::Configuration(value)
    }
}

/// Validates and exports a deterministic snapshot artifact.
pub fn export(snapshot: &MetadataSnapshot) -> Result<ExportArtifact, SnapshotError> {
    validate(snapshot)?;
    let mut bytes = serde_json::to_vec_pretty(snapshot)?;
    bytes.push(b'\n');
    Ok(ExportArtifact {
        checksum_sha256: checksum(&bytes),
        bytes,
    })
}

/// Restores a snapshot only after verifying its external checksum and schema.
pub fn restore(bytes: &[u8], expected_checksum: &str) -> Result<MetadataSnapshot, SnapshotError> {
    if !is_sha256(expected_checksum) || checksum(bytes) != expected_checksum {
        return Err(SnapshotError::ChecksumMismatch);
    }
    let snapshot: MetadataSnapshot = serde_json::from_slice(bytes)?;
    validate(&snapshot)?;
    Ok(snapshot)
}

/// Performs an idempotent copy-on-write migration of legacy metadata.
pub fn migrate_legacy(snapshot: &MetadataSnapshot) -> Result<MigrationOutput, SnapshotError> {
    let backup = export(snapshot)?;
    if snapshot.configuration.schema_version != INSTANCE_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchema);
    }
    Ok(MigrationOutput {
        backup,
        snapshot: snapshot.clone(),
        result: "legacy-compatible-no-rewrite",
    })
}

fn validate(snapshot: &MetadataSnapshot) -> Result<(), SnapshotError> {
    if snapshot.schema_version != SNAPSHOT_SCHEMA {
        return Err(SnapshotError::UnsupportedSchema);
    }
    snapshot.configuration.validate()?;
    if !safe_id(&snapshot.export_id) || !timestamp(&snapshot.exported_at) {
        return Err(SnapshotError::Invalid("export identity/time"));
    }
    if snapshot.catalogue.len() > 100_000 || snapshot.extension_pairings.len() > 10_000 {
        return Err(SnapshotError::Invalid("snapshot bounds"));
    }
    for record in &snapshot.catalogue {
        if !safe_id(&record.canonical_media_identity)
            || !safe_id(&record.endpoint_id)
            || !safe_id(&record.object_store_id)
            || !safe_id(&record.object_reference_id)
            || !is_sha256(&record.checksum_sha256)
            || record.historic_label.is_empty()
            || record.historic_label.len() > 128
        {
            return Err(SnapshotError::Invalid("catalogue record"));
        }
    }
    for pairing in &snapshot.extension_pairings {
        if !safe_id(&pairing.profile_id)
            || !pairing.instance_origin.starts_with("https://")
            || pairing.instance_origin.contains(['?', '#', '@'])
            || !pairing.requires_reviewed_repair
        {
            return Err(SnapshotError::Invalid("extension pairing"));
        }
    }
    Ok(())
}

fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
}

fn timestamp(value: &str) -> bool {
    value.len() >= 20 && value.ends_with('Z') && value.contains('T')
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn snapshot() -> MetadataSnapshot {
        let configuration = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/config/instance.v1.json"
        )))
        .unwrap();
        MetadataSnapshot {
            schema_version: SNAPSHOT_SCHEMA.into(),
            export_id: "export-fixture-1".into(),
            exported_at: "2026-07-15T12:00:00Z".into(),
            configuration,
            catalogue: vec![CatalogueSnapshotRecord {
                canonical_media_identity: "site:fixture:media-1".into(),
                endpoint_id: "fixture-endpoint".into(),
                object_store_id: "fixture-store".into(),
                object_reference_id: "object-1".into(),
                checksum_sha256: "a".repeat(64),
                historic_label: "x-img generic-site v1".into(),
            }],
            extension_pairings: vec![PairingSnapshotRecord {
                profile_id: "firefox-profile-1".into(),
                instance_origin: "https://x-img.example".into(),
                requires_reviewed_repair: true,
            }],
        }
    }

    #[test]
    fn export_restore_and_repeat_migration_preserve_all_authority_identities() {
        let original = snapshot();
        let first = migrate_legacy(&original).unwrap();
        let restored = restore(&first.backup.bytes, &first.backup.checksum_sha256).unwrap();
        let second = migrate_legacy(&restored).unwrap();
        assert_eq!(first.snapshot, original);
        assert_eq!(second.snapshot, original);
        assert_eq!(first.backup, second.backup);
        assert_eq!(restored.catalogue[0], original.catalogue[0]);
        assert!(restored.extension_pairings[0].requires_reviewed_repair);
    }

    #[test]
    fn corruption_future_major_unknown_fields_and_unreviewed_pairing_fail_closed() {
        let artifact = export(&snapshot()).unwrap();
        let mut corrupt = artifact.bytes.clone();
        corrupt[20] ^= 1;
        assert!(matches!(
            restore(&corrupt, &artifact.checksum_sha256),
            Err(SnapshotError::ChecksumMismatch)
        ));

        let mut future: Value = serde_json::from_slice(&artifact.bytes).unwrap();
        future["schema_version"] = "x-img.metadata-snapshot.v2".into();
        let future_bytes = serde_json::to_vec(&future).unwrap();
        assert!(matches!(
            restore(&future_bytes, &checksum(&future_bytes)),
            Err(SnapshotError::UnsupportedSchema)
        ));

        let mut unknown: Value = serde_json::from_slice(&artifact.bytes).unwrap();
        unknown["credential"] = "not-allowed".into();
        let unknown_bytes = serde_json::to_vec(&unknown).unwrap();
        assert!(matches!(
            restore(&unknown_bytes, &checksum(&unknown_bytes)),
            Err(SnapshotError::Json(_))
        ));

        let mut unsafe_pairing = snapshot();
        unsafe_pairing.extension_pairings[0].requires_reviewed_repair = false;
        assert!(matches!(
            export(&unsafe_pairing),
            Err(SnapshotError::Invalid("extension pairing"))
        ));
    }
}
