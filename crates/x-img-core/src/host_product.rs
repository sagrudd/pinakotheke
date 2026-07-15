// SPDX-License-Identifier: MPL-2.0
//! Strict, dependency-free validation for the x-img Monas product registration.
//!
//! This is a copied, versioned wire-contract shape, not a dependency on the
//! unpublished Monas workspace. A later host adapter is responsible for
//! validating the authenticated request context at runtime.

#![allow(missing_docs)]

use serde_json::Value;

pub const MONAS_PRODUCT_BOOTSTRAP_SCHEMA: &str = "x-img.monas-product-bootstrap.v1";

#[derive(Debug, Clone, Copy)]
struct ProductExpectation {
    product_id: &'static str,
    product_version: &'static str,
    product_root: &'static str,
    web_mount: &'static str,
    api_mount: &'static str,
    bootstrap_path: &'static str,
    visibility: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostProductError {
    Json(String),
    Invalid(String),
}

impl std::fmt::Display for HostProductError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) => {
                write!(formatter, "invalid product registration JSON: {message}")
            }
            Self::Invalid(message) => {
                write!(formatter, "invalid Monas product registration: {message}")
            }
        }
    }
}

impl std::error::Error for HostProductError {}

/// Parses and validates the public x-img Monas bootstrap registration.
///
/// It requires host-owned authentication for the one application and API mount,
/// a DASObjectStore requirement, and a bootstrap shape portable to a future
/// Synoptikon host. x-img login/session routes are not part of this contract.
pub fn validate_monas_product_bootstrap(bytes: &[u8]) -> Result<(), HostProductError> {
    validate_product_bootstrap(
        bytes,
        ProductExpectation {
            product_id: "x-img",
            product_version: env!("CARGO_PKG_VERSION"),
            product_root: "/opt/x-img",
            web_mount: "/products/x-img/app/",
            api_mount: "/products/x-img/api/",
            bootstrap_path: "/products/x-img/.well-known/mnemosyne/product-bootstrap.json",
            visibility: "local_profile_enabled",
        },
    )
}

/// Validates the inert Pinakotheke v1 Monas registration candidate.
pub fn validate_pinakotheke_product_candidate(bytes: &[u8]) -> Result<(), HostProductError> {
    validate_product_bootstrap(
        bytes,
        ProductExpectation {
            product_id: "pinakotheke",
            product_version: "1.0.0",
            product_root: "/opt/pinakotheke",
            web_mount: "/products/pinakotheke/app/",
            api_mount: "/products/pinakotheke/api/",
            bootstrap_path: "/products/pinakotheke/.well-known/mnemosyne/product-bootstrap.json",
            visibility: "cutover_candidate",
        },
    )
}

fn validate_product_bootstrap(
    bytes: &[u8],
    expected: ProductExpectation,
) -> Result<(), HostProductError> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|error| HostProductError::Json(error.to_string()))?;
    let object = document
        .as_object()
        .ok_or_else(|| HostProductError::Invalid("document must be an object".to_owned()))?;

    let allowed = [
        "schema_version",
        "product_id",
        "product_version",
        "host",
        "host_mode",
        "product_root",
        "web_mount",
        "api_mount",
        "bootstrap_path",
        "capabilities",
        "correlation_policy",
        "visibility",
        "external_sql_required",
        "object_store_required",
        "authentication_required",
        "authentication_framework",
        "device_token_requirement",
        "synoptikon_equivalent",
    ];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(HostProductError::Invalid(format!("unknown field `{key}`")));
        }
    }

    require_string(object, "schema_version", MONAS_PRODUCT_BOOTSTRAP_SCHEMA)?;
    require_string(object, "product_id", expected.product_id)?;
    require_string(object, "product_version", expected.product_version)?;
    require_string(object, "host", "monas")?;
    require_string(object, "host_mode", "monas_standalone")?;
    require_string(object, "product_root", expected.product_root)?;
    require_string(object, "web_mount", expected.web_mount)?;
    require_string(object, "api_mount", expected.api_mount)?;
    require_string(object, "bootstrap_path", expected.bootstrap_path)?;
    require_string(object, "correlation_policy", "host_generated")?;
    require_string(object, "visibility", expected.visibility)?;
    require_bool(object, "external_sql_required", false)?;
    require_bool(object, "object_store_required", true)?;
    require_bool(object, "authentication_required", true)?;
    require_string(object, "authentication_framework", "Prosopikon")?;
    require_string(object, "device_token_requirement", "NotRequired")?;

    let capabilities = require_array(object, "capabilities")?;
    for required in [
        "monas_mandatory_web_authentication",
        "dasobjectstore_required",
    ] {
        if !capabilities
            .iter()
            .any(|value| value.as_str() == Some(required))
        {
            return Err(HostProductError::Invalid(format!(
                "capabilities must include `{required}`"
            )));
        }
    }

    let equivalent = object
        .get("synoptikon_equivalent")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            HostProductError::Invalid("synoptikon_equivalent must be an object".to_owned())
        })?;
    if equivalent.len() != 3 {
        return Err(HostProductError::Invalid(
            "synoptikon_equivalent must contain exactly three fields".to_owned(),
        ));
    }
    require_string(
        equivalent,
        "bootstrap_schema",
        "mnemosyne.product_ui.bootstrap.v1",
    )?;
    require_string(equivalent, "host_context_owner", "host")?;
    let modes = require_array(equivalent, "host_modes")?;
    for required in ["monas_standalone", "synoptikon_integrated"] {
        if !modes.iter().any(|value| value.as_str() == Some(required)) {
            return Err(HostProductError::Invalid(format!(
                "synoptikon_equivalent.host_modes must include `{required}`"
            )));
        }
    }

    Ok(())
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), HostProductError> {
    match object.get(key).and_then(Value::as_str) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(HostProductError::Invalid(format!(
            "`{key}` must be `{expected}`, found `{value}`"
        ))),
        None => Err(HostProductError::Invalid(format!("`{key}` is required"))),
    }
}

fn require_bool(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: bool,
) -> Result<(), HostProductError> {
    match object.get(key).and_then(Value::as_bool) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(HostProductError::Invalid(format!(
            "`{key}` must be `{expected}`, found `{value}`"
        ))),
        None => Err(HostProductError::Invalid(format!("`{key}` is required"))),
    }
}

fn require_array<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, HostProductError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| HostProductError::Invalid(format!("`{key}` must be an array")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_versioned_product_registration() {
        validate_monas_product_bootstrap(include_bytes!(
            "../../../contracts/monas/x-img-product-bootstrap.v1.json"
        ))
        .expect("product registration must remain valid");
    }

    #[test]
    fn accepts_the_inert_pinakotheke_cutover_candidate() {
        validate_pinakotheke_product_candidate(include_bytes!(
            "../../../contracts/monas/pinakotheke-product-bootstrap.v1.candidate.json"
        ))
        .expect("candidate registration must remain valid");
    }

    #[test]
    fn rejects_an_anonymous_product_api() {
        let error = validate_monas_product_bootstrap(include_bytes!(
            "../../../fixtures/monas/v1/invalid-anonymous-api.json"
        ))
        .expect_err("anonymous API must be rejected");

        assert!(error.to_string().contains("authentication_required"));
    }

    #[test]
    fn rejects_a_direct_product_login_route() {
        let error = validate_monas_product_bootstrap(include_bytes!(
            "../../../fixtures/monas/v1/invalid-direct-login-route.json"
        ))
        .expect_err("product login route must be rejected");

        assert!(error.to_string().contains("direct_login_route"));
    }
}
