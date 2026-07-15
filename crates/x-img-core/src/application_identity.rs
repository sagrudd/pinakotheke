// SPDX-License-Identifier: MPL-2.0
//! Scoped, non-secret DASObjectStore application-identity registration checks.
//!
//! The daemon owns identity registration, proof verification, token issuance,
//! and credential custody. This module accepts only opaque references and
//! validates the x-img-side scope before a future storage adapter is invoked.

#![allow(missing_docs)]

use std::collections::BTreeSet;

use serde_json::Value;

pub const APPLICATION_IDENTITY_SCHEMA: &str = "x-img.das-application-identity.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedApplicationIdentity {
    endpoint_id: String,
    object_store_id: String,
    prefix: String,
    operations: BTreeSet<StorageOperation>,
    max_object_bytes: u64,
    max_total_bytes: u64,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageOperation {
    Read,
    Write,
    List,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageOperationRequest {
    operation_id: String,
    operation: StorageOperation,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationIdentityError {
    Json(String),
    Invalid(String),
    Expired,
    Replayed,
    WrongEndpoint,
    WrongStore,
    WrongPrefix,
    UnauthorizedOperation,
    Oversized,
}

impl std::fmt::Display for ApplicationIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) => {
                write!(formatter, "invalid application identity JSON: {message}")
            }
            Self::Invalid(message) => write!(formatter, "invalid application identity: {message}"),
            Self::Expired => formatter.write_str("application identity has expired"),
            Self::Replayed => formatter.write_str("storage operation was already authorized"),
            Self::WrongEndpoint => formatter.write_str("operation targets another endpoint"),
            Self::WrongStore => formatter.write_str("operation targets another ObjectStore"),
            Self::WrongPrefix => {
                formatter.write_str("operation object key is outside the allowed prefix")
            }
            Self::UnauthorizedOperation => {
                formatter.write_str("operation is outside the allowed scope")
            }
            Self::Oversized => formatter.write_str("operation exceeds the registered byte limit"),
        }
    }
}

impl std::error::Error for ApplicationIdentityError {}

impl ScopedApplicationIdentity {
    pub fn parse_registration(bytes: &[u8]) -> Result<Self, ApplicationIdentityError> {
        Self::parse_for(bytes, "x-img", true)
    }

    /// Parses the inert Pinakotheke service-principal cutover candidate.
    pub fn parse_pinakotheke_candidate(bytes: &[u8]) -> Result<Self, ApplicationIdentityError> {
        Self::parse_for(bytes, "pinakotheke", false)
    }

    fn parse_for(
        bytes: &[u8],
        expected_application_id: &str,
        expected_active: bool,
    ) -> Result<Self, ApplicationIdentityError> {
        let document: Value = serde_json::from_slice(bytes)
            .map_err(|error| ApplicationIdentityError::Json(error.to_string()))?;
        let object = document.as_object().ok_or_else(|| {
            ApplicationIdentityError::Invalid("document must be an object".to_owned())
        })?;
        let allowed = [
            "schema_version",
            "application_id",
            "owner_ref",
            "credential_ref",
            "endpoint_id",
            "object_store_id",
            "prefix",
            "operations",
            "max_object_bytes",
            "max_total_bytes",
            "issued_at_unix_seconds",
            "expires_at_unix_seconds",
            "active",
        ];
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ApplicationIdentityError::Invalid(format!(
                    "unknown field `{key}`"
                )));
            }
        }
        require_string(object, "schema_version", APPLICATION_IDENTITY_SCHEMA)?;
        require_string(object, "application_id", expected_application_id)?;
        let owner_ref = required_identifier(object, "owner_ref")?;
        if !owner_ref.starts_with("monas.host-context:") {
            return Err(ApplicationIdentityError::Invalid(
                "owner_ref must be a Monas host-context reference".to_owned(),
            ));
        }
        let credential_ref = required_identifier(object, "credential_ref")?;
        if !credential_ref.starts_with("dasobjectstore.application:") {
            return Err(ApplicationIdentityError::Invalid(
                "credential_ref must be an opaque DASObjectStore application reference".to_owned(),
            ));
        }
        let endpoint_id = required_identifier(object, "endpoint_id")?;
        let object_store_id = required_identifier(object, "object_store_id")?;
        let prefix = object
            .get("prefix")
            .and_then(Value::as_str)
            .filter(|value| is_safe_prefix(value))
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                ApplicationIdentityError::Invalid("`prefix` must be a safe prefix".to_owned())
            })?;
        let operation_values = object
            .get("operations")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ApplicationIdentityError::Invalid("`operations` must be an array".to_owned())
            })?;
        let operations = operation_values
            .iter()
            .map(parse_operation)
            .collect::<Result<BTreeSet<_>, _>>()?;
        if operations.is_empty()
            || operations.len() > 4
            || operations.len() != operation_values.len()
        {
            return Err(ApplicationIdentityError::Invalid(
                "operations must be a non-empty unique subset".to_owned(),
            ));
        }
        let max_object_bytes = required_u64(object, "max_object_bytes")?;
        let max_total_bytes = required_u64(object, "max_total_bytes")?;
        if max_object_bytes == 0 || max_total_bytes == 0 || max_object_bytes > max_total_bytes {
            return Err(ApplicationIdentityError::Invalid(
                "byte limits must be positive and object limit must not exceed total limit"
                    .to_owned(),
            ));
        }
        let issued_at_unix_seconds = required_u64(object, "issued_at_unix_seconds")?;
        let expires_at_unix_seconds = required_u64(object, "expires_at_unix_seconds")?;
        if expires_at_unix_seconds <= issued_at_unix_seconds {
            return Err(ApplicationIdentityError::Invalid(
                "identity expiry must follow issue time".to_owned(),
            ));
        }
        if object.get("active").and_then(Value::as_bool) != Some(expected_active) {
            return Err(ApplicationIdentityError::Invalid(format!(
                "identity active state must be `{expected_active}`"
            )));
        }
        Ok(Self {
            endpoint_id,
            object_store_id,
            prefix,
            operations,
            max_object_bytes,
            max_total_bytes,
            issued_at_unix_seconds,
            expires_at_unix_seconds,
        })
    }

    pub fn authorize_once(
        &self,
        request: &StorageOperationRequest,
        now_unix_seconds: u64,
        replay_registry: &mut BTreeSet<String>,
        reserved_bytes: &mut u64,
    ) -> Result<(), ApplicationIdentityError> {
        if now_unix_seconds < self.issued_at_unix_seconds
            || now_unix_seconds >= self.expires_at_unix_seconds
        {
            return Err(ApplicationIdentityError::Expired);
        }
        if replay_registry.contains(&request.operation_id) {
            return Err(ApplicationIdentityError::Replayed);
        }
        if request.endpoint_id != self.endpoint_id {
            return Err(ApplicationIdentityError::WrongEndpoint);
        }
        if request.object_store_id != self.object_store_id {
            return Err(ApplicationIdentityError::WrongStore);
        }
        if !request.object_key.starts_with(&self.prefix) {
            return Err(ApplicationIdentityError::WrongPrefix);
        }
        if !self.operations.contains(&request.operation) {
            return Err(ApplicationIdentityError::UnauthorizedOperation);
        }
        if request.size_bytes > self.max_object_bytes
            || reserved_bytes.saturating_add(request.size_bytes) > self.max_total_bytes
        {
            return Err(ApplicationIdentityError::Oversized);
        }
        replay_registry.insert(request.operation_id.clone());
        *reserved_bytes = reserved_bytes.saturating_add(request.size_bytes);
        Ok(())
    }
}

impl StorageOperationRequest {
    pub fn parse(bytes: &[u8]) -> Result<Self, ApplicationIdentityError> {
        let document: Value = serde_json::from_slice(bytes)
            .map_err(|error| ApplicationIdentityError::Json(error.to_string()))?;
        Self::from_value(&document)
    }

    fn from_value(value: &Value) -> Result<Self, ApplicationIdentityError> {
        let object = value.as_object().ok_or_else(|| {
            ApplicationIdentityError::Invalid("operation must be an object".to_owned())
        })?;
        let allowed = [
            "operation_id",
            "operation",
            "endpoint_id",
            "object_store_id",
            "object_key",
            "size_bytes",
        ];
        if object.len() != allowed.len() {
            return Err(ApplicationIdentityError::Invalid(
                "operation must contain exactly its scoped fields".to_owned(),
            ));
        }
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ApplicationIdentityError::Invalid(format!(
                    "unknown field `{key}`"
                )));
            }
        }
        Ok(Self {
            operation_id: required_identifier(object, "operation_id")?,
            operation: parse_operation(object.get("operation").ok_or_else(|| {
                ApplicationIdentityError::Invalid("`operation` is required".to_owned())
            })?)?,
            endpoint_id: required_identifier(object, "endpoint_id")?,
            object_store_id: required_identifier(object, "object_store_id")?,
            object_key: object
                .get("object_key")
                .and_then(Value::as_str)
                .filter(|value| is_safe_object_key(value))
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    ApplicationIdentityError::Invalid("`object_key` must be safe".to_owned())
                })?,
            size_bytes: required_u64(object, "size_bytes")?,
        })
    }
}

fn parse_operation(value: &Value) -> Result<StorageOperation, ApplicationIdentityError> {
    match value.as_str() {
        Some("read") => Ok(StorageOperation::Read),
        Some("write") => Ok(StorageOperation::Write),
        Some("list") => Ok(StorageOperation::List),
        Some("verify") => Ok(StorageOperation::Verify),
        _ => Err(ApplicationIdentityError::Invalid(
            "operation must be read, write, list, or verify".to_owned(),
        )),
    }
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), ApplicationIdentityError> {
    match object.get(key).and_then(Value::as_str) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(ApplicationIdentityError::Invalid(format!(
            "`{key}` must be `{expected}`, found `{value}`"
        ))),
        None => Err(ApplicationIdentityError::Invalid(format!(
            "`{key}` is required"
        ))),
    }
}

fn required_identifier(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, ApplicationIdentityError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| is_identifier(value))
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApplicationIdentityError::Invalid(format!("`{key}` must be an identifier")))
}

fn required_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<u64, ApplicationIdentityError> {
    object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        ApplicationIdentityError::Invalid(format!("`{key}` must be an unsigned integer"))
    })
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_safe_prefix(value: &str) -> bool {
    value.ends_with('/')
        && value.strip_suffix('/').is_some_and(is_safe_object_key)
        && !value.starts_with('/')
}

fn is_safe_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.contains("//")
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ScopedApplicationIdentity {
        ScopedApplicationIdentity::parse_registration(include_bytes!(
            "../../../contracts/dasobjectstore/x-img-application-identity.v1.json"
        ))
        .expect("registration must be valid")
    }

    #[test]
    fn pinakotheke_candidate_preserves_scope_without_claiming_activation() {
        let legacy = identity();
        let candidate = ScopedApplicationIdentity::parse_pinakotheke_candidate(include_bytes!(
            "../../../contracts/dasobjectstore/pinakotheke-application-identity.v1.candidate.json"
        ))
        .expect("candidate registration must be valid and inactive");

        assert_eq!(candidate.endpoint_id, legacy.endpoint_id);
        assert_eq!(candidate.object_store_id, legacy.object_store_id);
        assert_eq!(candidate.operations, legacy.operations);
        assert_eq!(candidate.max_object_bytes, legacy.max_object_bytes);
        assert_eq!(candidate.max_total_bytes, legacy.max_total_bytes);
        assert_eq!(candidate.prefix, "pinakotheke/");
        assert_eq!(legacy.prefix, "x-img/");
    }

    #[test]
    fn authorizes_only_the_registered_scope_and_rejects_replay() {
        let fixture: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/das-application/v1/authorization-cases.json"
        ))
        .expect("fixture must parse");
        assert_eq!(
            fixture["schema_version"],
            "x-img.das-application-fixtures.v1"
        );
        let cases = fixture["cases"]
            .as_array()
            .expect("fixture cases must be an array");
        let mut replay_registry = BTreeSet::new();
        let mut reserved_bytes = 0;

        let accepted = StorageOperationRequest::from_value(&cases[0]["operation"])
            .expect("accepted fixture operation must parse");
        identity()
            .authorize_once(&accepted, 500, &mut replay_registry, &mut reserved_bytes)
            .expect("registered operation must be authorized");
        assert_eq!(
            identity().authorize_once(&accepted, 500, &mut replay_registry, &mut reserved_bytes),
            Err(ApplicationIdentityError::Replayed)
        );
    }

    #[test]
    fn fixture_cases_fail_closed_for_expiry_scope_and_size() {
        let fixture: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/das-application/v1/authorization-cases.json"
        ))
        .expect("fixture must parse");
        let cases = fixture["cases"]
            .as_array()
            .expect("fixture cases must be an array");
        let expected = [
            ApplicationIdentityError::Expired,
            ApplicationIdentityError::WrongStore,
            ApplicationIdentityError::WrongPrefix,
            ApplicationIdentityError::Oversized,
        ];
        for (case, expected) in cases[1..].iter().zip(expected) {
            let operation = StorageOperationRequest::from_value(&case["operation"])
                .expect("fixture operation must parse");
            let now = case["now_unix_seconds"]
                .as_u64()
                .expect("fixture time must be u64");
            let mut replay_registry = BTreeSet::new();
            let mut reserved_bytes = 0;
            assert_eq!(
                identity().authorize_once(
                    &operation,
                    now,
                    &mut replay_registry,
                    &mut reserved_bytes
                ),
                Err(expected),
                "fixture case {}",
                case["id"]
            );
        }
    }
}
