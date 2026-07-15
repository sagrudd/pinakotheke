// SPDX-License-Identifier: MPL-2.0
//! Approved deletion and compliance reconciliation.
//!
//! This metadata-only state machine never deletes an object itself. It keeps
//! catalogue visibility separate from a scoped DASObjectStore removal request
//! and accepts completion only after matching authority evidence.

use crate::acquisition::VerifiedObject;

/// Approved reason for hiding or removing a committed record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceReason {
    /// The source reports that its item was deleted.
    SourceDeleted,
    /// The actor no longer has permission to retain or display the item.
    AccessRevoked,
    /// A rights decision requires removal.
    RightsWithdrawn,
    /// The owning user explicitly requested removal.
    UserRequested,
}

/// Approved scope of a compliance action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceScope {
    /// Remove catalogue visibility but retain the authority object.
    CatalogueOnly,
    /// Tombstone the catalogue and request exact authority-object removal.
    CatalogueAndObject,
}

/// Host-approved, non-secret evidence authorizing one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceApproval {
    /// Stable policy-decision reference.
    pub policy_decision_id: String,
    /// Opaque actor reference, never a session or token.
    pub actor_ref: String,
    /// RFC 3339 approval time.
    pub approved_at: String,
}

/// Explicit request bound to one committed immutable object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceRequest {
    /// Stable request identity used for replay deduplication.
    pub request_id: String,
    /// Stable catalogue/media identity.
    pub canonical_media_identity: String,
    /// Reason reviewed by the policy authority.
    pub reason: ComplianceReason,
    /// Reviewed removal scope.
    pub scope: ComplianceScope,
    /// Exact committed object evidence.
    pub object: VerifiedObject,
    /// Required host approval.
    pub approval: ComplianceApproval,
}

/// Explicit lifecycle state; no state implies authority deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceState {
    /// Record remains normally visible.
    Active,
    /// Catalogue record is hidden and retains its provenance.
    Tombstoned,
    /// Exact object removal was submitted to the authority.
    RemovalRequested,
    /// DASObjectStore verified that the exact object is absent/removed.
    RemovalVerified,
    /// Authority evidence conflicts with the reviewed request.
    Conflict,
}

/// Observation returned by an authorized DASObjectStore adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalObservation {
    /// Authority has not reached a terminal result.
    Pending,
    /// Exact object is verified removed or absent under the request capability.
    Removed(VerifiedObject),
    /// Exact object remains present; retry remains permitted.
    StillPresent(VerifiedObject),
}

/// One bounded, redacted audit fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceAuditEvent {
    /// Stable event code; contains no free-form provider payload.
    pub code: &'static str,
    /// State after the event.
    pub state: ComplianceState,
}

/// Result of idempotent reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// Catalogue-only action is complete.
    CatalogueTombstoned,
    /// Authority removal still needs submission or verification.
    AwaitingAuthority,
    /// Exact authority removal is verified.
    RemovalVerified,
    /// Immutable authority evidence does not match the reviewed object.
    Conflict,
}

/// Metadata lifecycle for one approved action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceReconciliation {
    request: ComplianceRequest,
    state: ComplianceState,
    audit: Vec<ComplianceAuditEvent>,
}

/// Reconciliation rejected before mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceError {
    /// Required identity or approval evidence is unsafe/missing.
    InvalidEvidence,
    /// Object removal was submitted for a catalogue-only approval.
    ScopeNotApproved,
    /// Operation is invalid for the current state.
    InvalidTransition,
}

impl ComplianceReconciliation {
    /// Validates an approved request without changing catalogue visibility.
    pub fn approved(request: ComplianceRequest) -> Result<Self, ComplianceError> {
        validate_request(&request)?;
        Ok(Self {
            request,
            state: ComplianceState::Active,
            audit: vec![ComplianceAuditEvent {
                code: "compliance.approved",
                state: ComplianceState::Active,
            }],
        })
    }

    /// Tombstones catalogue visibility. Replays are idempotent.
    pub fn tombstone(&mut self) -> Result<ReconcileOutcome, ComplianceError> {
        match self.state {
            ComplianceState::Active => {
                self.state = ComplianceState::Tombstoned;
                self.push_once("compliance.catalogue-tombstoned");
            }
            ComplianceState::Tombstoned
            | ComplianceState::RemovalRequested
            | ComplianceState::RemovalVerified
            | ComplianceState::Conflict => {}
        }
        Ok(if self.request.scope == ComplianceScope::CatalogueOnly {
            ReconcileOutcome::CatalogueTombstoned
        } else {
            self.outcome()
        })
    }

    /// Records submission of the exact authority deletion request.
    pub fn mark_removal_requested(&mut self) -> Result<(), ComplianceError> {
        if self.request.scope != ComplianceScope::CatalogueAndObject {
            return Err(ComplianceError::ScopeNotApproved);
        }
        match self.state {
            ComplianceState::Tombstoned => {
                self.state = ComplianceState::RemovalRequested;
                self.push_once("compliance.object-removal-requested");
                Ok(())
            }
            ComplianceState::RemovalRequested => Ok(()),
            _ => Err(ComplianceError::InvalidTransition),
        }
    }

    /// Reconciles authority evidence without guessing from provider state.
    pub fn reconcile(
        &mut self,
        observation: RemovalObservation,
    ) -> Result<ReconcileOutcome, ComplianceError> {
        if self.request.scope != ComplianceScope::CatalogueAndObject {
            return Err(ComplianceError::ScopeNotApproved);
        }
        if !matches!(
            self.state,
            ComplianceState::RemovalRequested
                | ComplianceState::RemovalVerified
                | ComplianceState::Conflict
        ) {
            return Err(ComplianceError::InvalidTransition);
        }
        if self.state == ComplianceState::RemovalVerified || self.state == ComplianceState::Conflict
        {
            return Ok(self.outcome());
        }
        match observation {
            RemovalObservation::Pending => Ok(ReconcileOutcome::AwaitingAuthority),
            RemovalObservation::StillPresent(object) => {
                if object_matches(&self.request.object, &object) {
                    Ok(ReconcileOutcome::AwaitingAuthority)
                } else {
                    self.conflict()
                }
            }
            RemovalObservation::Removed(object) => {
                if !object_matches(&self.request.object, &object) {
                    return self.conflict();
                }
                self.state = ComplianceState::RemovalVerified;
                self.push_once("compliance.object-removal-verified");
                Ok(ReconcileOutcome::RemovalVerified)
            }
        }
    }

    /// Current word-first state.
    #[must_use]
    pub const fn state(&self) -> ComplianceState {
        self.state
    }

    /// Exact approved request retained for provenance.
    #[must_use]
    pub const fn request(&self) -> &ComplianceRequest {
        &self.request
    }

    /// Append-only bounded audit evidence.
    #[must_use]
    pub fn audit(&self) -> &[ComplianceAuditEvent] {
        &self.audit
    }

    fn conflict(&mut self) -> Result<ReconcileOutcome, ComplianceError> {
        self.state = ComplianceState::Conflict;
        self.push_once("compliance.authority-conflict");
        Ok(ReconcileOutcome::Conflict)
    }

    fn outcome(&self) -> ReconcileOutcome {
        match self.state {
            ComplianceState::RemovalVerified => ReconcileOutcome::RemovalVerified,
            ComplianceState::Conflict => ReconcileOutcome::Conflict,
            _ => ReconcileOutcome::AwaitingAuthority,
        }
    }

    fn push_once(&mut self, code: &'static str) {
        if self.audit.last().is_none_or(|event| event.code != code) {
            self.audit.push(ComplianceAuditEvent {
                code,
                state: self.state,
            });
        }
    }
}

fn validate_request(request: &ComplianceRequest) -> Result<(), ComplianceError> {
    if !safe_id(&request.request_id)
        || !safe_id(&request.canonical_media_identity)
        || !safe_id(&request.approval.policy_decision_id)
        || !safe_id(&request.approval.actor_ref)
        || !timestamp(&request.approval.approved_at)
    {
        return Err(ComplianceError::InvalidEvidence);
    }
    VerifiedObject::new(
        request.object.endpoint_id.clone(),
        request.object.object_store_id.clone(),
        request.object.object_reference_id.clone(),
        request.object.checksum_sha256.clone(),
    )
    .map_err(|_| ComplianceError::InvalidEvidence)?;
    Ok(())
}

fn object_matches(expected: &VerifiedObject, observed: &VerifiedObject) -> bool {
    expected == observed
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
    use super::*;

    fn request(scope: ComplianceScope) -> ComplianceRequest {
        ComplianceRequest {
            request_id: "remove-1".into(),
            canonical_media_identity: "site:fixture:media-1".into(),
            reason: ComplianceReason::RightsWithdrawn,
            scope,
            object: VerifiedObject::new("endpoint-1", "store-1", "object-1", "a".repeat(64))
                .unwrap(),
            approval: ComplianceApproval {
                policy_decision_id: "decision-1".into(),
                actor_ref: "actor-1".into(),
                approved_at: "2026-07-15T12:00:00Z".into(),
            },
        }
    }

    #[test]
    fn catalogue_only_tombstones_without_authority_deletion() {
        let mut item =
            ComplianceReconciliation::approved(request(ComplianceScope::CatalogueOnly)).unwrap();
        assert_eq!(
            item.tombstone().unwrap(),
            ReconcileOutcome::CatalogueTombstoned
        );
        assert_eq!(
            item.tombstone().unwrap(),
            ReconcileOutcome::CatalogueTombstoned
        );
        assert_eq!(item.state(), ComplianceState::Tombstoned);
        assert_eq!(item.audit().len(), 2);
        assert_eq!(
            item.mark_removal_requested(),
            Err(ComplianceError::ScopeNotApproved)
        );
    }

    #[test]
    fn approved_object_removal_reconciles_once_across_pending_and_replay() {
        let mut item =
            ComplianceReconciliation::approved(request(ComplianceScope::CatalogueAndObject))
                .unwrap();
        item.tombstone().unwrap();
        item.mark_removal_requested().unwrap();
        item.mark_removal_requested().unwrap();
        assert_eq!(
            item.reconcile(RemovalObservation::Pending).unwrap(),
            ReconcileOutcome::AwaitingAuthority
        );
        assert_eq!(
            item.reconcile(RemovalObservation::StillPresent(
                item.request.object.clone()
            ))
            .unwrap(),
            ReconcileOutcome::AwaitingAuthority
        );
        assert_eq!(
            item.reconcile(RemovalObservation::Removed(item.request.object.clone()))
                .unwrap(),
            ReconcileOutcome::RemovalVerified
        );
        assert_eq!(
            item.reconcile(RemovalObservation::Removed(item.request.object.clone()))
                .unwrap(),
            ReconcileOutcome::RemovalVerified
        );
        assert_eq!(item.audit().len(), 4);
    }

    #[test]
    fn mismatched_authority_object_conflicts_and_never_claims_removal() {
        let mut item =
            ComplianceReconciliation::approved(request(ComplianceScope::CatalogueAndObject))
                .unwrap();
        item.tombstone().unwrap();
        item.mark_removal_requested().unwrap();
        let other =
            VerifiedObject::new("endpoint-1", "store-2", "object-1", "a".repeat(64)).unwrap();
        assert_eq!(
            item.reconcile(RemovalObservation::Removed(other)).unwrap(),
            ReconcileOutcome::Conflict
        );
        assert_eq!(item.state(), ComplianceState::Conflict);
        assert_ne!(item.state(), ComplianceState::RemovalVerified);
    }

    #[test]
    fn missing_approval_and_delete_before_tombstone_fail_before_mutation() {
        let mut invalid = request(ComplianceScope::CatalogueAndObject);
        invalid.approval.policy_decision_id.clear();
        assert_eq!(
            ComplianceReconciliation::approved(invalid),
            Err(ComplianceError::InvalidEvidence)
        );
        let mut item =
            ComplianceReconciliation::approved(request(ComplianceScope::CatalogueAndObject))
                .unwrap();
        assert_eq!(
            item.mark_removal_requested(),
            Err(ComplianceError::InvalidTransition)
        );
        assert_eq!(item.state(), ComplianceState::Active);
    }
}
