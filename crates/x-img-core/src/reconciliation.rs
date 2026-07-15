// SPDX-License-Identifier: MPL-2.0
//! Deterministic idempotency and crash-reconciliation rules.
//!
//! This module retains only bounded metadata in memory. It never reads media
//! bytes, calls DASObjectStore, stores a capability, or treats an observed URL
//! as identity. A future authorized adapter supplies a reconciliation
//! observation; this core converges that observation without overwriting an
//! existing committed object.

use std::collections::{BTreeMap, BTreeSet};

use crate::acquisition::VerifiedObject;

/// Immutable idempotency key: canonical media identity plus checksum.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SettlementKey {
    /// Stable platform/site resource identity, never a URL.
    pub canonical_media_identity: String,
    /// Immutable lowercase SHA-256 of the verified object.
    pub checksum_sha256: String,
}

impl SettlementKey {
    /// Creates a safe, bounded idempotency key.
    pub fn new(
        canonical_media_identity: impl Into<String>,
        checksum_sha256: impl Into<String>,
    ) -> Result<Self, ReconciliationError> {
        let key = Self {
            canonical_media_identity: canonical_media_identity.into(),
            checksum_sha256: checksum_sha256.into(),
        };
        key.validate()?;
        Ok(key)
    }

    fn validate(&self) -> Result<(), ReconciliationError> {
        if !is_safe_identity(&self.canonical_media_identity) {
            return Err(ReconciliationError::InvalidMetadata {
                field: "canonical_media_identity",
            });
        }
        if !is_sha256(&self.checksum_sha256) {
            return Err(ReconciliationError::InvalidMetadata {
                field: "checksum_sha256",
            });
        }
        Ok(())
    }
}

/// A bounded reconciliation request supplied after an interrupted acquisition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationRequest {
    /// Canonical identity expected by the acquisition attempt.
    pub canonical_media_identity: String,
    /// Checksum expected by the attempt before catalogue settlement.
    pub expected_checksum_sha256: String,
    /// Safe source URL aliases observed for the same identity.
    pub source_url_aliases: BTreeSet<String>,
}

impl ReconciliationRequest {
    /// Creates a request while rejecting URLs used as identity or unsafe aliases.
    pub fn new(
        canonical_media_identity: impl Into<String>,
        expected_checksum_sha256: impl Into<String>,
        source_url_aliases: impl IntoIterator<Item = String>,
    ) -> Result<Self, ReconciliationError> {
        let request = Self {
            canonical_media_identity: canonical_media_identity.into(),
            expected_checksum_sha256: expected_checksum_sha256.into(),
            source_url_aliases: source_url_aliases.into_iter().collect(),
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), ReconciliationError> {
        SettlementKey::new(
            self.canonical_media_identity.clone(),
            self.expected_checksum_sha256.clone(),
        )?;
        if self.source_url_aliases.len() > 32 {
            return Err(ReconciliationError::InvalidMetadata {
                field: "source_url_aliases",
            });
        }
        if self
            .source_url_aliases
            .iter()
            .any(|alias| !is_safe_alias(alias))
        {
            return Err(ReconciliationError::InvalidMetadata {
                field: "source_url_aliases",
            });
        }
        Ok(())
    }
}

/// Result supplied by a future authorized DASObjectStore reconciliation adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityObservation {
    /// No verified authority object is currently available for the attempt.
    Absent,
    /// The authority reports a verified immutable object.
    Verified(VerifiedObject),
    /// The authority reports immutable object evidence with an unexpected checksum.
    ChecksumMismatch(VerifiedObject),
}

/// One committed metadata record retained by the in-memory catalogue model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRecord {
    /// Immutable key used to deduplicate retries and replays.
    pub settlement_key: SettlementKey,
    /// The first verified authority object admitted for this key.
    pub object: VerifiedObject,
    /// Append-only safe aliases observed for this same identity.
    pub source_url_aliases: BTreeSet<String>,
}

/// The deterministic outcome of reconciling one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationOutcome {
    /// No authority object was verified; no catalogue mutation occurred.
    AwaitingAuthority,
    /// A new verified object was committed to the metadata catalogue.
    Committed {
        /// Immutable settlement key created by this reconciliation.
        key: SettlementKey,
    },
    /// A retry/replay matched a prior commit and may have added aliases.
    AlreadyCommitted {
        /// Immutable settlement key matched by this replay.
        key: SettlementKey,
    },
    /// Conflicting checksum evidence was retained as a conflict, never overwritten.
    Conflict {
        /// Canonical identity shared by the conflicting evidence.
        canonical_media_identity: String,
        /// Expected checksum from the interrupted attempt.
        expected_checksum_sha256: String,
        /// Checksum observed from the authority or prior commit.
        observed_checksum_sha256: String,
    },
}

/// In-memory metadata catalogue used by deterministic reconciliation tests and ports.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconciliationCatalogue {
    committed: BTreeMap<SettlementKey, CommittedRecord>,
    canonical_checksums: BTreeMap<String, String>,
    conflicts: BTreeMap<String, BTreeSet<String>>,
}

impl ReconciliationCatalogue {
    /// Reconciles one authority observation without any storage or network call.
    pub fn reconcile(
        &mut self,
        request: ReconciliationRequest,
        observation: AuthorityObservation,
    ) -> Result<ReconciliationOutcome, ReconciliationError> {
        request.validate()?;
        match observation {
            AuthorityObservation::Absent => Ok(ReconciliationOutcome::AwaitingAuthority),
            AuthorityObservation::Verified(object) => self.reconcile_verified(request, object),
            AuthorityObservation::ChecksumMismatch(object) => {
                validate_object(&object)?;
                self.reconcile_mismatch(request, object.checksum_sha256)
            }
        }
    }

    /// Returns a committed record for the exact immutable settlement key.
    #[must_use]
    pub fn committed(&self, key: &SettlementKey) -> Option<&CommittedRecord> {
        self.committed.get(key)
    }

    /// Returns the number of committed records; retries never increase it.
    #[must_use]
    pub fn committed_len(&self) -> usize {
        self.committed.len()
    }

    /// Finds the canonical identity already associated with a safe source alias.
    ///
    /// A browser capture can reuse this identity rather than creating a second
    /// catalogue record when an account connector already committed the same
    /// canonical media URL with verified object evidence.
    #[must_use]
    pub fn canonical_identity_for_alias(&self, alias: &str) -> Option<&str> {
        self.committed
            .values()
            .find(|record| record.source_url_aliases.contains(alias))
            .map(|record| record.settlement_key.canonical_media_identity.as_str())
    }

    /// Returns immutable checksum evidence retained for a conflicting identity.
    #[must_use]
    pub fn conflict_checksums(&self, canonical_media_identity: &str) -> Option<&BTreeSet<String>> {
        self.conflicts.get(canonical_media_identity)
    }

    fn reconcile_verified(
        &mut self,
        request: ReconciliationRequest,
        object: VerifiedObject,
    ) -> Result<ReconciliationOutcome, ReconciliationError> {
        validate_object(&object)?;
        if object.checksum_sha256 != request.expected_checksum_sha256 {
            return self.reconcile_mismatch(request, object.checksum_sha256);
        }
        let key = SettlementKey::new(
            request.canonical_media_identity.clone(),
            request.expected_checksum_sha256,
        )?;
        if let Some(existing_checksum) = self.canonical_checksums.get(&key.canonical_media_identity)
            && existing_checksum != &key.checksum_sha256
        {
            return Ok(self.register_conflict(
                key.canonical_media_identity,
                existing_checksum.clone(),
                key.checksum_sha256,
            ));
        }
        if let Some(record) = self.committed.get_mut(&key) {
            record.source_url_aliases.extend(request.source_url_aliases);
            return Ok(ReconciliationOutcome::AlreadyCommitted { key });
        }
        self.canonical_checksums.insert(
            key.canonical_media_identity.clone(),
            key.checksum_sha256.clone(),
        );
        self.committed.insert(
            key.clone(),
            CommittedRecord {
                settlement_key: key.clone(),
                object,
                source_url_aliases: request.source_url_aliases,
            },
        );
        Ok(ReconciliationOutcome::Committed { key })
    }

    fn reconcile_mismatch(
        &mut self,
        request: ReconciliationRequest,
        observed_checksum_sha256: String,
    ) -> Result<ReconciliationOutcome, ReconciliationError> {
        if !is_sha256(&observed_checksum_sha256) {
            return Err(ReconciliationError::InvalidMetadata {
                field: "observed_checksum_sha256",
            });
        }
        Ok(self.register_conflict(
            request.canonical_media_identity,
            request.expected_checksum_sha256,
            observed_checksum_sha256,
        ))
    }

    fn register_conflict(
        &mut self,
        canonical_media_identity: String,
        expected_checksum_sha256: String,
        observed_checksum_sha256: String,
    ) -> ReconciliationOutcome {
        let checksums = self
            .conflicts
            .entry(canonical_media_identity.clone())
            .or_default();
        checksums.insert(expected_checksum_sha256.clone());
        checksums.insert(observed_checksum_sha256.clone());
        ReconciliationOutcome::Conflict {
            canonical_media_identity,
            expected_checksum_sha256,
            observed_checksum_sha256,
        }
    }
}

/// Error returned before the reconciliation catalogue can be mutated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationError {
    /// Bounded metadata failed the public contract shape.
    InvalidMetadata {
        /// The invalid field name.
        field: &'static str,
    },
}

impl std::fmt::Display for ReconciliationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadata { field } => {
                write!(formatter, "invalid reconciliation metadata: {field}")
            }
        }
    }
}

impl std::error::Error for ReconciliationError {}

fn validate_object(object: &VerifiedObject) -> Result<(), ReconciliationError> {
    if !is_safe_identity(&object.endpoint_id) {
        return Err(ReconciliationError::InvalidMetadata {
            field: "endpoint_id",
        });
    }
    if !is_safe_identity(&object.object_store_id) {
        return Err(ReconciliationError::InvalidMetadata {
            field: "object_store_id",
        });
    }
    if !is_safe_identity(&object.object_reference_id) {
        return Err(ReconciliationError::InvalidMetadata {
            field: "object_reference_id",
        });
    }
    if !is_sha256(&object.checksum_sha256) {
        return Err(ReconciliationError::InvalidMetadata {
            field: "checksum_sha256",
        });
    }
    Ok(())
}

fn is_safe_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
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
    let Some(host_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    !host_and_path.is_empty()
        && value.len() <= 2048
        && !host_and_path.contains(['@', '?', '#', ' ', '\n', '\r'])
        && !host_and_path.starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::{
        AuthorityObservation, ReconciliationCatalogue, ReconciliationOutcome,
        ReconciliationRequest, SettlementKey,
    };
    use crate::acquisition::VerifiedObject;

    const CHECKSUM_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CHECKSUM_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn object(checksum: &str, object_reference_id: &str) -> VerifiedObject {
        VerifiedObject::new(
            "fixture-endpoint",
            "fixture-store",
            object_reference_id,
            checksum,
        )
        .expect("synthetic verified object")
    }

    fn request(checksum: &str, aliases: &[&str]) -> ReconciliationRequest {
        ReconciliationRequest::new(
            "x:fixture-account:post-1:media-1",
            checksum,
            aliases.iter().map(|alias| (*alias).to_owned()),
        )
        .expect("synthetic reconciliation request")
    }

    #[test]
    fn crash_replays_converge_to_one_committed_record_without_overwrite() {
        let mut catalogue = ReconciliationCatalogue::default();
        let first = catalogue
            .reconcile(
                request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                AuthorityObservation::Verified(object(CHECKSUM_A, "fixture-object-first")),
            )
            .expect("first settlement");
        assert!(matches!(first, ReconciliationOutcome::Committed { .. }));

        let replay = catalogue
            .reconcile(
                request(CHECKSUM_A, &["https://x.example.invalid/post/1"]),
                AuthorityObservation::Verified(object(CHECKSUM_A, "fixture-object-second")),
            )
            .expect("replay settlement");
        assert!(matches!(
            replay,
            ReconciliationOutcome::AlreadyCommitted { .. }
        ));

        let key = SettlementKey::new("x:fixture-account:post-1:media-1", CHECKSUM_A)
            .expect("settlement key");
        let record = catalogue.committed(&key).expect("one record");
        assert_eq!(catalogue.committed_len(), 1);
        assert_eq!(record.object.object_reference_id, "fixture-object-first");
        assert_eq!(record.source_url_aliases.len(), 2);
    }

    #[test]
    fn crash_before_or_without_authority_verification_does_not_mutate_catalogue() {
        let mut catalogue = ReconciliationCatalogue::default();
        let outcome = catalogue
            .reconcile(
                request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                AuthorityObservation::Absent,
            )
            .expect("absence is a valid observation");
        assert_eq!(outcome, ReconciliationOutcome::AwaitingAuthority);
        assert_eq!(catalogue.committed_len(), 0);
    }

    #[test]
    fn crash_boundaries_converge_without_creating_duplicate_commits() {
        for pre_verification_state in ["discovered", "claimed", "transferring", "stored"] {
            let mut catalogue = ReconciliationCatalogue::default();
            let outcome = catalogue
                .reconcile(
                    request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                    AuthorityObservation::Absent,
                )
                .expect("pre-verification crash observation");
            assert_eq!(
                outcome,
                ReconciliationOutcome::AwaitingAuthority,
                "{pre_verification_state}"
            );
            assert_eq!(catalogue.committed_len(), 0, "{pre_verification_state}");
        }

        for post_verification_state in ["verified-before-commit", "around-catalogue-commit"] {
            let mut catalogue = ReconciliationCatalogue::default();
            let first = catalogue
                .reconcile(
                    request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                    AuthorityObservation::Verified(object(CHECKSUM_A, "fixture-object")),
                )
                .expect("post-verification recovery");
            let replay = catalogue
                .reconcile(
                    request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                    AuthorityObservation::Verified(object(CHECKSUM_A, "fixture-object")),
                )
                .expect("post-verification replay");
            assert!(
                matches!(first, ReconciliationOutcome::Committed { .. }),
                "{post_verification_state}"
            );
            assert!(
                matches!(replay, ReconciliationOutcome::AlreadyCommitted { .. }),
                "{post_verification_state}"
            );
            assert_eq!(catalogue.committed_len(), 1, "{post_verification_state}");
        }
    }

    #[test]
    fn checksum_mismatch_and_platform_id_reuse_become_conflicts() {
        let mut catalogue = ReconciliationCatalogue::default();
        let mismatch = catalogue
            .reconcile(
                request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                AuthorityObservation::ChecksumMismatch(object(CHECKSUM_B, "fixture-object-b")),
            )
            .expect("mismatch is recorded as a conflict");
        assert!(matches!(mismatch, ReconciliationOutcome::Conflict { .. }));
        assert_eq!(catalogue.committed_len(), 0);

        catalogue
            .reconcile(
                request(CHECKSUM_A, &["https://x.example.invalid/status/1"]),
                AuthorityObservation::Verified(object(CHECKSUM_A, "fixture-object-a")),
            )
            .expect("first immutable object commits");
        let reuse = catalogue
            .reconcile(
                request(CHECKSUM_B, &["https://x.example.invalid/status/1"]),
                AuthorityObservation::Verified(object(CHECKSUM_B, "fixture-object-b")),
            )
            .expect("reused identity is a conflict");
        assert!(matches!(reuse, ReconciliationOutcome::Conflict { .. }));
        assert_eq!(catalogue.committed_len(), 1);
        assert_eq!(
            catalogue
                .conflict_checksums("x:fixture-account:post-1:media-1")
                .expect("conflict evidence")
                .len(),
            2
        );
    }

    #[test]
    fn rejects_url_identity_and_unsafe_aliases() {
        assert!(
            ReconciliationRequest::new("https://x.example.invalid/status/1", CHECKSUM_A, [])
                .is_err()
        );
        assert!(
            ReconciliationRequest::new(
                "x:fixture-account:post-1:media-1",
                CHECKSUM_A,
                ["https://x.example.invalid/status/1?token=not-allowed".to_owned()],
            )
            .is_err()
        );
    }
}
