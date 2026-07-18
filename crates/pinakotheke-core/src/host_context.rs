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
    authority_id: Option<String>,
    authorizations: BTreeSet<String>,
    correlation_id: String,
    host_mode: HostMode,
    principal_id: Option<String>,
    session_id: Option<String>,
    synoptikon_scope: Option<SynoptikonScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynoptikonScope {
    tenant_id: String,
    account_id: String,
    project_id: String,
    entitlement_id: String,
}

impl SynoptikonScope {
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn entitlement_id(&self) -> &str {
        &self.entitlement_id
    }
}

impl AuthenticatedHostContext {
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub fn authority_id(&self) -> Option<&str> {
        self.authority_id.as_deref()
    }

    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    pub const fn host_mode(&self) -> HostMode {
        self.host_mode
    }

    pub fn principal_id(&self) -> Option<&str> {
        self.principal_id.as_deref()
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn permits(&self, authorization: &str) -> bool {
        self.authorizations.contains(authorization)
    }

    pub fn synoptikon_scope(&self) -> Option<&SynoptikonScope> {
        self.synoptikon_scope.as_ref()
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
        "authority_id",
        "audience",
        "principal_id",
        "session_id",
        "authorizations",
        "correlation_id",
        "tenant_id",
        "account_id",
        "project_id",
        "entitlement_id",
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
    let canonical_identity = parse_canonical_identity(object)?;
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

    let synoptikon_scope = match host_mode {
        HostMode::MonasStandalone => {
            for key in ["tenant_id", "account_id", "project_id", "entitlement_id"] {
                if object.contains_key(key) {
                    return Err(HostContextError::Invalid(format!(
                        "`{key}` is not valid in Monas standalone context"
                    )));
                }
            }
            None
        }
        HostMode::SynoptikonIntegrated => Some(SynoptikonScope {
            tenant_id: required_identifier(object, "tenant_id")?,
            account_id: required_identifier(object, "account_id")?,
            project_id: required_identifier(object, "project_id")?,
            entitlement_id: required_identifier(object, "entitlement_id")?,
        }),
    };

    if !authorizations.contains(XIMG_ACCESS) {
        return Err(HostContextError::Unauthorized);
    }
    Ok(AuthenticatedHostContext {
        actor_id,
        authority_id: canonical_identity
            .as_ref()
            .map(|identity| identity.0.clone()),
        authorizations,
        correlation_id,
        host_mode,
        principal_id: canonical_identity
            .as_ref()
            .map(|identity| identity.1.clone()),
        session_id: canonical_identity.map(|identity| identity.2),
        synoptikon_scope,
    })
}

fn parse_canonical_identity(
    object: &serde_json::Map<String, Value>,
) -> Result<Option<(String, String, String)>, HostContextError> {
    const FIELDS: [&str; 4] = ["authority_id", "audience", "principal_id", "session_id"];
    let present = FIELDS
        .iter()
        .filter(|key| object.contains_key(**key))
        .count();
    if present == 0 {
        // Explicit compatibility for extension pairings that still emit the
        // original host-context v1 shape during the coordinated rollout.
        return Ok(None);
    }
    if present != FIELDS.len() {
        return Err(HostContextError::Invalid(
            "canonical identity fields must be supplied together".to_owned(),
        ));
    }

    let authority_id = required_uuid(object, "authority_id")?;
    require_string(object, "audience", "pinakotheke")?;
    let principal_id = required_uuid(object, "principal_id")?;
    let session_id = required_uuid(object, "session_id")?;
    Ok(Some((authority_id, principal_id, session_id)))
}

fn required_uuid(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, HostContextError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| is_uuid(value))
        .map(ToOwned::to_owned)
        .ok_or_else(|| HostContextError::Invalid(format!("`{key}` must be a UUID")))
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value != "00000000-0000-0000-0000-000000000000"
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'),
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
        assert_eq!(
            context
                .synoptikon_scope()
                .expect("project scope")
                .project_id(),
            "synthetic-project"
        );
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

    #[test]
    fn canonical_prosopikon_identity_is_accepted() {
        let context = MonasHostContextAdapter
            .authenticate(
                br#"{
                    "schema_version":"x-img.host-context.v1",
                    "host":"monas",
                    "host_mode":"monas_standalone",
                    "actor_id":"synthetic-monas-user",
                    "authority_id":"8f61b404-9b83-4d7f-9f55-30dc8705ce95",
                    "audience":"pinakotheke",
                    "principal_id":"c45a4b5c-6ec8-4e47-9ea5-446a0741f650",
                    "session_id":"a48c2f17-8df7-40af-a018-e5022d6bc21f",
                    "authorizations":["ximg.access"],
                    "correlation_id":"canonical-context-test"
                }"#,
            )
            .expect("canonical Prosopikon identity must be accepted");

        assert_eq!(
            context.authority_id(),
            Some("8f61b404-9b83-4d7f-9f55-30dc8705ce95")
        );
        assert_eq!(
            context.principal_id(),
            Some("c45a4b5c-6ec8-4e47-9ea5-446a0741f650")
        );
        assert_eq!(
            context.session_id(),
            Some("a48c2f17-8df7-40af-a018-e5022d6bc21f")
        );
    }

    #[test]
    fn canonical_identity_rejects_wrong_audience_partial_fields_and_invalid_uuids() {
        let base = r#"{
            "schema_version":"x-img.host-context.v1",
            "host":"monas",
            "host_mode":"monas_standalone",
            "actor_id":"synthetic-monas-user",
            "authority_id":"8f61b404-9b83-4d7f-9f55-30dc8705ce95",
            "audience":"AUDIENCE",
            "principal_id":"c45a4b5c-6ec8-4e47-9ea5-446a0741f650",
            "session_id":"a48c2f17-8df7-40af-a018-e5022d6bc21f",
            "authorizations":["ximg.access"],
            "correlation_id":"canonical-context-test"
        }"#;

        for (document, expected) in [
            (
                base.replace("AUDIENCE", "pinakotheke").replace(
                    "8f61b404-9b83-4d7f-9f55-30dc8705ce95",
                    "00000000-0000-0000-0000-000000000000",
                ),
                "`authority_id` must be a UUID",
            ),
            (
                base.replace("AUDIENCE", "x-img"),
                "`audience` must be `pinakotheke`",
            ),
            (
                base.replace("AUDIENCE", "pinakotheke").replace(
                    ",\n            \"session_id\":\"a48c2f17-8df7-40af-a018-e5022d6bc21f\"",
                    "",
                ),
                "canonical identity fields must be supplied together",
            ),
        ] {
            let error = MonasHostContextAdapter
                .authenticate(document.as_bytes())
                .expect_err("invalid canonical identity must fail closed");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}
