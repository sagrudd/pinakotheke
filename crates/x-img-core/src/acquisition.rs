// SPDX-License-Identifier: MPL-2.0
//! Explicit, storage-free acquisition lifecycle rules.
//!
//! The state machine accepts only metadata supplied by future adapters. It does
//! not perform a transfer, call DASObjectStore, persist bytes, or mark an
//! authority reference as authentic. A caller may commit only after supplying
//! verified object evidence, and may admit a review state only after commit.

/// Media acquisition lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionState {
    /// A candidate was identified but is not owned by a worker.
    Discovered,
    /// One worker holds a claim for the candidate.
    Claimed,
    /// A claimed candidate is being transferred through an authority boundary.
    Transferring,
    /// The authority accepted an object but verification is still required.
    Stored,
    /// Immutable ObjectStore evidence has been verified.
    Verified,
    /// Verified evidence is admitted to the catalogue.
    Committed,
    /// An acquisition attempt failed explicitly.
    Failed,
    /// Policy or rights rules prohibit further acquisition.
    PolicyBlocked,
    /// A user or scheduler cancelled work before settlement.
    Cancelled,
    /// A committed or inaccessible record was explicitly tombstoned.
    Tombstoned,
    /// Conflicting immutable evidence prevents settlement.
    Conflict,
}

/// Catalogue review state for committed media.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// Present a committed object for review.
    New,
    /// A user reviewed the committed object.
    Reviewed,
    /// A user retained the committed object.
    Retained,
    /// Hide the committed object without deleting it.
    Hidden,
    /// Mark the committed object removed from normal presentation.
    Removed,
}

/// Immutable object evidence supplied after an authority-side verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedObject {
    /// Stable endpoint or appliance identifier.
    pub endpoint_id: String,
    /// Stable logical ObjectStore identifier.
    pub object_store_id: String,
    /// Stable catalogue object-reference identifier.
    pub object_reference_id: String,
    /// Immutable lowercase SHA-256 checksum.
    pub checksum_sha256: String,
}

impl VerifiedObject {
    /// Creates verified metadata after checking safe identifier/checksum shape.
    pub fn new(
        endpoint_id: impl Into<String>,
        object_store_id: impl Into<String>,
        object_reference_id: impl Into<String>,
        checksum_sha256: impl Into<String>,
    ) -> Result<Self, AcquisitionError> {
        let object = Self {
            endpoint_id: endpoint_id.into(),
            object_store_id: object_store_id.into(),
            object_reference_id: object_reference_id.into(),
            checksum_sha256: checksum_sha256.into(),
        };
        object.validate()?;
        Ok(object)
    }

    fn validate(&self) -> Result<(), AcquisitionError> {
        for (field, value) in [
            ("endpoint_id", self.endpoint_id.as_str()),
            ("object_store_id", self.object_store_id.as_str()),
            ("object_reference_id", self.object_reference_id.as_str()),
        ] {
            if !is_identifier(value) {
                return Err(AcquisitionError::InvalidEvidence { field });
            }
        }
        if self.checksum_sha256.len() != 64
            || !self
                .checksum_sha256
                .chars()
                .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
        {
            return Err(AcquisitionError::InvalidEvidence {
                field: "checksum_sha256",
            });
        }
        Ok(())
    }
}

/// One in-memory media acquisition lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acquisition {
    media_identity_id: String,
    state: AcquisitionState,
    claim_id: Option<String>,
    verified_object: Option<VerifiedObject>,
    review_state: Option<ReviewState>,
}

impl Acquisition {
    /// Starts a newly discovered acquisition for a stable media identity.
    pub fn discovered(media_identity_id: impl Into<String>) -> Result<Self, AcquisitionError> {
        let media_identity_id = media_identity_id.into();
        if !is_identifier(&media_identity_id) {
            return Err(AcquisitionError::InvalidEvidence {
                field: "media_identity_id",
            });
        }
        Ok(Self {
            media_identity_id,
            state: AcquisitionState::Discovered,
            claim_id: None,
            verified_object: None,
            review_state: None,
        })
    }

    /// Returns the canonical media identity for this lifecycle.
    #[must_use]
    pub fn media_identity_id(&self) -> &str {
        &self.media_identity_id
    }

    /// Returns the current explicit acquisition state.
    #[must_use]
    pub const fn state(&self) -> AcquisitionState {
        self.state
    }

    /// Returns the claimed lease identifier, if the item is currently claimed.
    #[must_use]
    pub fn claim_id(&self) -> Option<&str> {
        self.claim_id.as_deref()
    }

    /// Returns immutable verified object evidence after successful verification.
    #[must_use]
    pub fn verified_object(&self) -> Option<&VerifiedObject> {
        self.verified_object.as_ref()
    }

    /// Returns the review state after catalogue commit.
    #[must_use]
    pub const fn review_state(&self) -> Option<ReviewState> {
        self.review_state
    }

    /// Claims a discovered item exactly once until a future reconciliation rule releases it.
    pub fn claim(&mut self, claim_id: impl Into<String>) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Discovered, "claim")?;
        let claim_id = claim_id.into();
        if !is_identifier(&claim_id) {
            return Err(AcquisitionError::InvalidEvidence { field: "claim_id" });
        }
        self.claim_id = Some(claim_id);
        self.state = AcquisitionState::Claimed;
        Ok(())
    }

    /// Starts a transfer only after an exclusive claim.
    pub fn start_transfer(&mut self) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Claimed, "start_transfer")?;
        self.state = AcquisitionState::Transferring;
        Ok(())
    }

    /// Records that an authority accepted an object pending verification.
    pub fn record_stored(&mut self) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Transferring, "record_stored")?;
        self.state = AcquisitionState::Stored;
        Ok(())
    }

    /// Attaches immutable authority-verified object evidence.
    pub fn verify(&mut self, object: VerifiedObject) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Stored, "verify")?;
        object.validate()?;
        self.verified_object = Some(object);
        self.state = AcquisitionState::Verified;
        Ok(())
    }

    /// Commits only a verified ObjectStore object to the catalogue.
    pub fn commit(&mut self) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Verified, "commit")?;
        if self.verified_object.is_none() {
            return Err(AcquisitionError::MissingVerifiedObject);
        }
        self.state = AcquisitionState::Committed;
        Ok(())
    }

    /// Applies a review outcome only after a verified object was committed.
    pub fn set_review_state(&mut self, review_state: ReviewState) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Committed, "set_review_state")?;
        if self.verified_object.is_none() {
            return Err(AcquisitionError::MissingVerifiedObject);
        }
        self.review_state = Some(review_state);
        Ok(())
    }

    /// Records an explicit failure before settlement.
    pub fn fail(&mut self) -> Result<(), AcquisitionError> {
        self.transition_to_terminal(AcquisitionState::Failed, "fail")
    }

    /// Records a policy/rights block before settlement.
    pub fn block_policy(&mut self) -> Result<(), AcquisitionError> {
        self.transition_to_terminal(AcquisitionState::PolicyBlocked, "block_policy")
    }

    /// Records cancellation before settlement.
    pub fn cancel(&mut self) -> Result<(), AcquisitionError> {
        self.transition_to_terminal(AcquisitionState::Cancelled, "cancel")
    }

    /// Records a conflicting immutable identity or object evidence outcome.
    pub fn conflict(&mut self) -> Result<(), AcquisitionError> {
        self.transition_to_terminal(AcquisitionState::Conflict, "conflict")
    }

    /// Tombstones a committed record after policy, deletion, or access handling.
    pub fn tombstone(&mut self) -> Result<(), AcquisitionError> {
        self.require(AcquisitionState::Committed, "tombstone")?;
        self.state = AcquisitionState::Tombstoned;
        Ok(())
    }

    fn transition_to_terminal(
        &mut self,
        destination: AcquisitionState,
        operation: &'static str,
    ) -> Result<(), AcquisitionError> {
        if is_terminal(self.state) {
            return Err(AcquisitionError::InvalidTransition {
                operation,
                state: self.state,
            });
        }
        self.state = destination;
        Ok(())
    }

    fn require(
        &self,
        expected: AcquisitionState,
        operation: &'static str,
    ) -> Result<(), AcquisitionError> {
        if self.state == expected {
            Ok(())
        } else {
            Err(AcquisitionError::InvalidTransition {
                operation,
                state: self.state,
            })
        }
    }
}

/// State-machine error that contains no credentials, URLs, or media bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquisitionError {
    /// An operation did not apply in the current lifecycle state.
    InvalidTransition {
        /// Attempted operation name.
        operation: &'static str,
        /// State in which it was attempted.
        state: AcquisitionState,
    },
    /// Required object evidence was absent.
    MissingVerifiedObject,
    /// Metadata was malformed before it could become acquisition evidence.
    InvalidEvidence {
        /// Name of the invalid metadata field.
        field: &'static str,
    },
}

impl std::fmt::Display for AcquisitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { operation, state } => {
                write!(
                    formatter,
                    "cannot {operation} while acquisition is {state:?}"
                )
            }
            Self::MissingVerifiedObject => {
                formatter.write_str("verified ObjectStore evidence is required")
            }
            Self::InvalidEvidence { field } => {
                write!(formatter, "invalid acquisition evidence: {field}")
            }
        }
    }
}

impl std::error::Error for AcquisitionError {}

fn is_terminal(state: AcquisitionState) -> bool {
    matches!(
        state,
        AcquisitionState::Committed
            | AcquisitionState::Failed
            | AcquisitionState::PolicyBlocked
            | AcquisitionState::Cancelled
            | AcquisitionState::Tombstoned
            | AcquisitionState::Conflict
    )
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | ':' | '-')
        })
}

#[cfg(test)]
mod tests {
    use super::{Acquisition, AcquisitionState, ReviewState, VerifiedObject};

    fn evidence() -> VerifiedObject {
        VerifiedObject::new(
            "fixture-endpoint",
            "fixture-store",
            "fixture-object",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .expect("synthetic evidence is valid")
    }

    fn discovered() -> Acquisition {
        Acquisition::discovered("fixture-media").expect("synthetic identity is valid")
    }

    #[test]
    fn accepts_the_complete_lifecycle_and_review_after_commit() {
        let mut acquisition = discovered();
        acquisition.claim("fixture-lease").expect("claim");
        acquisition.start_transfer().expect("transfer");
        acquisition.record_stored().expect("stored");
        acquisition.verify(evidence()).expect("verified");
        acquisition.commit().expect("committed");
        acquisition
            .set_review_state(ReviewState::New)
            .expect("review after commit");

        assert_eq!(acquisition.state(), AcquisitionState::Committed);
        assert_eq!(acquisition.review_state(), Some(ReviewState::New));
        assert_eq!(
            acquisition
                .verified_object()
                .expect("evidence")
                .object_store_id,
            "fixture-store"
        );
    }

    #[test]
    fn rejects_double_claims_and_all_out_of_order_happy_path_steps() {
        let mut acquisition = discovered();
        assert!(acquisition.start_transfer().is_err());
        assert!(acquisition.record_stored().is_err());
        assert!(acquisition.verify(evidence()).is_err());
        assert!(acquisition.commit().is_err());
        acquisition.claim("fixture-lease").expect("first claim");
        assert!(acquisition.claim("another-lease").is_err());
        assert!(acquisition.commit().is_err());
        acquisition.start_transfer().expect("transfer");
        assert!(acquisition.commit().is_err());
        acquisition.record_stored().expect("stored");
        assert!(acquisition.commit().is_err());
    }

    #[test]
    fn rejects_review_before_verified_object_commit() {
        let mut acquisition = discovered();
        assert!(acquisition.set_review_state(ReviewState::New).is_err());
        acquisition.claim("fixture-lease").expect("claim");
        acquisition.start_transfer().expect("transfer");
        acquisition.record_stored().expect("stored");
        acquisition.verify(evidence()).expect("verified");
        assert!(acquisition.set_review_state(ReviewState::Reviewed).is_err());
        acquisition.commit().expect("commit");
        acquisition
            .set_review_state(ReviewState::Reviewed)
            .expect("review after commit");
    }

    #[test]
    fn terminal_outcomes_cannot_reenter_the_happy_path() {
        for terminal in [
            AcquisitionState::Failed,
            AcquisitionState::PolicyBlocked,
            AcquisitionState::Cancelled,
            AcquisitionState::Conflict,
        ] {
            let mut acquisition = discovered();
            match terminal {
                AcquisitionState::Failed => acquisition.fail().expect("fail"),
                AcquisitionState::PolicyBlocked => acquisition.block_policy().expect("block"),
                AcquisitionState::Cancelled => acquisition.cancel().expect("cancel"),
                AcquisitionState::Conflict => acquisition.conflict().expect("conflict"),
                _ => unreachable!("only terminal outcomes are enumerated"),
            }
            assert_eq!(acquisition.state(), terminal);
            assert!(acquisition.claim("fixture-lease").is_err());
            assert!(acquisition.start_transfer().is_err());
            assert!(acquisition.commit().is_err());
        }
    }

    #[test]
    fn rejects_malformed_verified_evidence() {
        assert!(
            VerifiedObject::new(
                "fixture-endpoint",
                "fixture-store",
                "fixture-object",
                "not-a-checksum"
            )
            .is_err()
        );
        assert!(Acquisition::discovered("@not-an-identity").is_err());
    }
}
