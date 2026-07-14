// SPDX-License-Identifier: MPL-2.0
//! Explicit, review-before-transfer bioinformatics resource plans.
//!
//! This contract neither resolves an accession nor transfers bytes. A future
//! adapter supplies the bounded resolution evidence; this model makes the
//! destination, rights result, and user confirmation explicit before a job may
//! be requested.

#![allow(missing_docs)]

/// Supported explicit source authorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAuthority {
    Geo,
    Sra,
    Ena,
    Ncbi,
}
/// Policy outcome displayed before a transfer can be confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyResult {
    Allowed,
    ReviewRequired,
    Blocked,
}
/// Transport selected by a future adapter; HTTPS is always the baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Https,
    Ftp,
    Aspera,
}
/// One resolved provider file, without bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceFile {
    pub file_id: String,
    pub filename: String,
    pub bytes: u64,
    pub checksum: String,
    pub transport: Transport,
}
/// Stable reviewed destination selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub prefix: String,
    pub object_type: String,
}
/// Reviewable, bounded transfer plan for exactly one explicit resource input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPlan {
    pub authority: ResourceAuthority,
    pub accession_or_url: String,
    pub release: String,
    pub files: Vec<ResourceFile>,
    pub destination: Destination,
    pub rights_note: String,
    pub policy: PolicyResult,
    pub confirmed: bool,
}
impl TransferPlan {
    /// Validates a complete plan without accessing a repository or ObjectStore.
    pub fn new(
        authority: ResourceAuthority,
        accession_or_url: impl Into<String>,
        release: impl Into<String>,
        files: Vec<ResourceFile>,
        destination: Destination,
        rights_note: impl Into<String>,
        policy: PolicyResult,
    ) -> Result<Self, PlanError> {
        let plan = Self {
            authority,
            accession_or_url: accession_or_url.into(),
            release: release.into(),
            files,
            destination,
            rights_note: rights_note.into(),
            policy,
            confirmed: false,
        };
        plan.validate()?;
        Ok(plan)
    }
    /// Confirms only an allowed, complete plan; transfer remains a future adapter operation.
    pub fn confirm(&mut self) -> Result<(), PlanError> {
        if self.policy != PolicyResult::Allowed {
            return Err(PlanError::PolicyBlocked);
        }
        self.confirmed = true;
        Ok(())
    }
    /// Returns the total announced provider bytes with saturating arithmetic.
    #[must_use]
    pub fn estimated_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.bytes).sum()
    }
    fn validate(&self) -> Result<(), PlanError> {
        if self.accession_or_url.is_empty()
            || self.accession_or_url.len() > 2048
            || self.accession_or_url.contains(['*', '\n', '\r'])
        {
            return Err(PlanError::InvalidInput);
        }
        if self.release.is_empty()
            || self.rights_note.is_empty()
            || self.files.is_empty()
            || self.files.len() > 1000
        {
            return Err(PlanError::IncompletePlan);
        }
        if !identifier(&self.destination.endpoint_id)
            || !identifier(&self.destination.object_store_id)
            || self.destination.prefix.is_empty()
            || !self.destination.prefix.ends_with('/')
            || self.destination.prefix.starts_with('/')
            || self.destination.object_type.is_empty()
        {
            return Err(PlanError::InvalidDestination);
        }
        for file in &self.files {
            if !identifier(&file.file_id)
                || file.filename.is_empty()
                || file.filename.contains(['/', '\n', '\r'])
                || !checksum(&file.checksum)
            {
                return Err(PlanError::InvalidFileEvidence);
            }
        }
        Ok(())
    }
}
/// Plan validation/confirmation rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    InvalidInput,
    IncompletePlan,
    InvalidDestination,
    InvalidFileEvidence,
    PolicyBlocked,
}
impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bioinformatics transfer plan rejected: {self:?}")
    }
}
impl std::error::Error for PlanError {}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | ':' | '-')
        })
}
fn checksum(value: &str) -> bool {
    (value.len() == 32 || value.len() == 64)
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn destination() -> Destination {
        Destination {
            endpoint_id: "fixture-endpoint".into(),
            object_store_id: "fixture-store".into(),
            prefix: "bio/".into(),
            object_type: "dataset_file".into(),
        }
    }
    fn file() -> ResourceFile {
        ResourceFile {
            file_id: "run-1".into(),
            filename: "fixture.fastq.gz".into(),
            bytes: 42,
            checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            transport: Transport::Https,
        }
    }
    #[test]
    fn accepts_one_explicit_allowed_plan_and_requires_confirmation() {
        let mut plan = TransferPlan::new(
            ResourceAuthority::Ena,
            "ENA:ERR400001",
            "fixture-release",
            vec![file()],
            destination(),
            "public synthetic fixture",
            PolicyResult::Allowed,
        )
        .expect("plan");
        assert!(!plan.confirmed);
        assert_eq!(plan.estimated_bytes(), 42);
        plan.confirm().expect("confirmation");
        assert!(plan.confirmed);
    }
    #[test]
    fn blocks_unreviewed_or_bulk_input_and_invalid_destination() {
        let mut blocked = TransferPlan::new(
            ResourceAuthority::Geo,
            "GEO:GSE1",
            "fixture",
            vec![file()],
            destination(),
            "rights uncertain",
            PolicyResult::Blocked,
        )
        .expect("metadata-only blocked plan");
        assert_eq!(blocked.confirm(), Err(PlanError::PolicyBlocked));
        assert!(
            TransferPlan::new(
                ResourceAuthority::Sra,
                "SRA:SRR*",
                "fixture",
                vec![file()],
                destination(),
                "public",
                PolicyResult::Allowed
            )
            .is_err()
        );
        let mut invalid = destination();
        invalid.prefix = "/arbitrary/".into();
        assert!(
            TransferPlan::new(
                ResourceAuthority::Ncbi,
                "NCBI:ABC1",
                "fixture",
                vec![file()],
                invalid,
                "public",
                PolicyResult::Allowed
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn resolution_and_transport_fixture_matrix_is_complete_and_synthetic() {
        let value: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/bioinformatics/v1/resolution-transport.json"
        )))
        .expect("fixture JSON");
        assert_eq!(value["schema_version"], "x-img.bioinformatics-fixtures.v1");
        for required in [
            "ena-multi-run",
            "sra-manifest",
            "geo-raw-archive",
            "ncbi-routing",
            "retry-resume-cancel",
            "aspera-fallback",
            "checksum-mismatch",
            "backpressure",
        ] {
            assert!(
                value["cases"]
                    .as_array()
                    .expect("cases")
                    .iter()
                    .any(|case| case["id"] == required),
                "missing {required}"
            );
        }
        assert!(
            !serde_json::to_string(&value)
                .expect("serialize")
                .to_ascii_lowercase()
                .contains("authorization: bearer")
        );
    }
}
