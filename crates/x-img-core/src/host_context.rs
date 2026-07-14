// SPDX-License-Identifier: MPL-2.0
//! Host-validated identity and authorization context for privileged x-img APIs.
//!
//! The input to these adapters is intentionally post-authentication host
//! metadata. It has no cookie, password, session token, or credential field.
//! Monas or Synoptikon validates those secrets before it creates this context.

#![allow(missing_docs)]

use std::collections::BTreeSet;

use serde_json::Value;

pub const HOST_CONTEXT_SCHEMA: &str = "x-img.host-context.v1";
pub const XIMG_ACCESS: &str = "ximg.access";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMode {
    MonasStandalone,
    SynoptikonIntegrated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedHostContext {
    actor_id: String,
    authorizations: BTreeSet<String>,
    correlation_id: String,
    host_mode: HostMode,
}

impl AuthenticatedHostContext {
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    pub const fn host_mode(&self) -> HostMode {
        self.host_mode
    }

    pub fn permits(&self, authorization: &str) -> bool {
        self.authorizations.contains(authorization)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostContextError {
    Json(String),
    Invalid(String),
    Unauthorized,
}

impl std::fmt::Display for HostContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) => write!(formatter, "invalid host context JSON: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid host context: {message}"),
            Self::Unauthorized => formatter.write_str("host context lacks x-img access"),
        }
    }
}

impl std::error::Error for HostContextError {}

/// Accepts a context only after the named host has authenticated it.
pub trait HostContextAdapter {
    fn authenticate(
        &self,
        verified_host_context: &[u8],
    ) -> Result<AuthenticatedHostContext, HostContextError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MonasHostContextAdapter;

impl HostContextAdapter for MonasHostContextAdapter {
    fn authenticate(
        &self,
        verified_host_context: &[u8],
    ) -> Result<AuthenticatedHostContext, HostContextError> {
        parse_verified_context(verified_host_context, "monas", HostMode::MonasStandalone)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SynoptikonHostContextAdapter;

impl HostContextAdapter for SynoptikonHostContextAdapter {
    fn authenticate(
        &self,
        verified_host_context: &[u8],
    ) -> Result<AuthenticatedHostContext, HostContextError> {
        parse_verified_context(
            verified_host_context,
            "synoptikon",
            HostMode::SynoptikonIntegrated,
        )
    }
}

fn parse_verified_context(
    bytes: &[u8],
    host: &str,
    host_mode: HostMode,
) -> Result<AuthenticatedHostContext, HostContextError> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|error| HostContextError::Json(error.to_string()))?;
    let object = document
        .as_object()
        .ok_or_else(|| HostContextError::Invalid("document must be an object".to_owned()))?;
    let allowed = [
        "schema_version",
        "host",
        "host_mode",
        "actor_id",
        "authorizations",
        "correlation_id",
    ];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(HostContextError::Invalid(format!("unknown field `{key}`")));
        }
    }

    require_string(object, "schema_version", HOST_CONTEXT_SCHEMA)?;
    require_string(object, "host", host)?;
    require_string(
        object,
        "host_mode",
        match host_mode {
            HostMode::MonasStandalone => "monas_standalone",
            HostMode::SynoptikonIntegrated => "synoptikon_integrated",
        },
    )?;
    let actor_id = required_identifier(object, "actor_id")?;
    let correlation_id = required_identifier(object, "correlation_id")?;
    let authorizations = object
        .get("authorizations")
        .and_then(Value::as_array)
        .ok_or_else(|| HostContextError::Invalid("`authorizations` must be an array".to_owned()))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| is_identifier(item))
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    HostContextError::Invalid(
                        "`authorizations` must contain non-secret identifiers".to_owned(),
                    )
                })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;

    if !authorizations.contains(XIMG_ACCESS) {
        return Err(HostContextError::Unauthorized);
    }
    Ok(AuthenticatedHostContext {
        actor_id,
        authorizations,
        correlation_id,
        host_mode,
    })
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), HostContextError> {
    match object.get(key).and_then(Value::as_str) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(HostContextError::Invalid(format!(
            "`{key}` must be `{expected}`, found `{value}`"
        ))),
        None => Err(HostContextError::Invalid(format!("`{key}` is required"))),
    }
}

fn required_identifier(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, HostContextError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| is_identifier(value))
        .map(ToOwned::to_owned)
        .ok_or_else(|| HostContextError::Invalid(format!("`{key}` must be an identifier")))
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monas_context_is_authorized_without_a_session_secret() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("Monas-validated context must be accepted");

        assert_eq!(context.actor_id(), "synthetic-monas-user");
        assert_eq!(context.host_mode(), HostMode::MonasStandalone);
        assert!(context.permits(XIMG_ACCESS));
    }

    #[test]
    fn synoptikon_can_replace_the_monas_adapter() {
        let context = SynoptikonHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/synoptikon-valid.json"
            ))
            .expect("Synoptikon-validated context must be accepted");

        assert_eq!(context.actor_id(), "synthetic-synoptikon-user");
        assert_eq!(context.host_mode(), HostMode::SynoptikonIntegrated);
        assert!(context.permits("ximg.review"));
    }

    #[test]
    fn context_without_product_authorization_fails_closed() {
        let error = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/invalid-without-access.json"
            ))
            .expect_err("host context requires x-img access");

        assert_eq!(error, HostContextError::Unauthorized);
    }
}
