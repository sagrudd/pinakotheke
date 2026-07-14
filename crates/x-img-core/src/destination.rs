// SPDX-License-Identifier: MPL-2.0
//! Versioned endpoint/device and logical ObjectStore selection contract.
//!
//! This is an x-img-owned, metadata-only validation boundary. DASObjectStore
//! remains authoritative for bootstrap, pairing, discovery, credential custody,
//! health, quota, and the final write-capability check.

#![allow(missing_docs)]

use serde_json::Value;

pub const DESTINATION_INVENTORY_SCHEMA: &str = "x-img.das-destination-inventory.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedDestination {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_type: String,
    pub selection_kind: String,
    pub reviewed_at_unix_seconds: u64,
    pub actor_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinationError {
    Json(String),
    Invalid(String),
    DestinationUnavailable(String),
}

impl std::fmt::Display for DestinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) => write!(f, "invalid destination JSON: {message}"),
            Self::Invalid(message) => write!(f, "invalid destination inventory: {message}"),
            Self::DestinationUnavailable(message) => {
                write!(f, "reviewed destination is unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for DestinationError {}

/// Validates an authority-discovered inventory and returns one explicitly
/// reviewed stable-ID destination. It intentionally cannot choose a store.
pub fn validate_reviewed_destination(
    bytes: &[u8],
) -> Result<ReviewedDestination, DestinationError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| DestinationError::Json(error.to_string()))?;
    let root = value
        .as_object()
        .ok_or_else(|| DestinationError::Invalid("document must be an object".to_owned()))?;
    reject_unknown(
        root,
        &[
            "schema_version",
            "actor_ref",
            "endpoints",
            "reviewed_destination",
        ],
    )?;
    require_exact(root, "schema_version", DESTINATION_INVENTORY_SCHEMA)?;
    let actor_ref = host_ref(root, "actor_ref")?;
    let endpoints = root
        .get("endpoints")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            DestinationError::Invalid("`endpoints` must be a non-empty array".to_owned())
        })?;
    let reviewed = root
        .get("reviewed_destination")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            DestinationError::Invalid("`reviewed_destination` must be an object".to_owned())
        })?;
    reject_unknown(
        reviewed,
        &[
            "endpoint_id",
            "object_store_id",
            "object_type",
            "selection_kind",
            "reviewed_at_unix_seconds",
            "actor_ref",
        ],
    )?;
    let result = ReviewedDestination {
        endpoint_id: identifier(reviewed, "endpoint_id")?,
        object_store_id: identifier(reviewed, "object_store_id")?,
        object_type: one_of(
            reviewed,
            "object_type",
            &["image", "video", "bioinformatics"],
        )?,
        selection_kind: one_of(
            reviewed,
            "selection_kind",
            &["endpoint_default", "site_override", "resource_override"],
        )?,
        reviewed_at_unix_seconds: number(reviewed, "reviewed_at_unix_seconds")?,
        actor_ref: host_ref(reviewed, "actor_ref")?,
    };
    if result.actor_ref != actor_ref {
        return Err(DestinationError::Invalid(
            "reviewed destination actor must match inventory actor".to_owned(),
        ));
    }
    let endpoint = endpoints
        .iter()
        .filter_map(Value::as_object)
        .find(|endpoint| {
            endpoint.get("endpoint_id").and_then(Value::as_str) == Some(result.endpoint_id.as_str())
        })
        .ok_or_else(|| {
            DestinationError::DestinationUnavailable("endpoint ID was not discovered".to_owned())
        })?;
    validate_endpoint(endpoint)?;
    if endpoint.get("health").and_then(Value::as_str) != Some("ready")
        || endpoint.get("tls_state").and_then(Value::as_str) != Some("trusted")
    {
        return Err(DestinationError::DestinationUnavailable(
            "endpoint is not ready with trusted TLS".to_owned(),
        ));
    }
    let default = endpoint
        .get("default_object_store_id")
        .and_then(Value::as_str);
    if result.selection_kind == "endpoint_default"
        && default != Some(result.object_store_id.as_str())
    {
        return Err(DestinationError::Invalid(
            "endpoint-default selection must name the endpoint's explicit default ObjectStore"
                .to_owned(),
        ));
    }
    let stores = endpoint
        .get("object_stores")
        .and_then(Value::as_array)
        .expect("validated endpoint stores");
    let store = stores
        .iter()
        .filter_map(Value::as_object)
        .find(|store| {
            store.get("object_store_id").and_then(Value::as_str)
                == Some(result.object_store_id.as_str())
        })
        .ok_or_else(|| {
            DestinationError::DestinationUnavailable(
                "ObjectStore ID was not discovered on endpoint".to_owned(),
            )
        })?;
    if store.get("writable").and_then(Value::as_bool) != Some(true)
        || store.get("health").and_then(Value::as_str) != Some("ready")
        || number(store, "quota_available_bytes")? == 0
        || !store
            .get("object_types")
            .and_then(Value::as_array)
            .is_some_and(|types| {
                types
                    .iter()
                    .any(|item| item.as_str() == Some(result.object_type.as_str()))
            })
    {
        return Err(DestinationError::DestinationUnavailable(
            "ObjectStore is not a compatible writable ready destination".to_owned(),
        ));
    }
    Ok(result)
}

fn validate_endpoint(endpoint: &serde_json::Map<String, Value>) -> Result<(), DestinationError> {
    reject_unknown(
        endpoint,
        &[
            "endpoint_id",
            "display_name",
            "deployment",
            "service_url",
            "provisioning_ref",
            "pairing_ref",
            "tls_state",
            "health",
            "default_object_store_id",
            "object_stores",
        ],
    )?;
    identifier(endpoint, "endpoint_id")?;
    nonempty(endpoint, "display_name")?;
    let deployment = one_of(
        endpoint,
        "deployment",
        &["local_folder_profile", "remote_appliance"],
    )?;
    let url = nonempty(endpoint, "service_url")?;
    if !url.starts_with("https://") {
        return Err(DestinationError::Invalid(
            "endpoint service_url must use HTTPS".to_owned(),
        ));
    }
    one_of(
        endpoint,
        "tls_state",
        &["trusted", "needs_trust", "revoked"],
    )?;
    one_of(
        endpoint,
        "health",
        &["ready", "read_only", "unavailable", "needs_reconnect"],
    )?;
    match deployment.as_str() {
        "local_folder_profile" => {
            opaque_or_null(endpoint, "provisioning_ref", "dasobjectstore.profile:")?;
            require_null(endpoint, "pairing_ref")?;
        }
        "remote_appliance" => {
            require_null(endpoint, "provisioning_ref")?;
            opaque_or_null(endpoint, "pairing_ref", "dasobjectstore.pairing:")?;
        }
        _ => unreachable!(),
    }
    let stores = endpoint
        .get("object_stores")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            DestinationError::Invalid("endpoint object_stores must be non-empty".to_owned())
        })?;
    for store in stores {
        validate_store(store.as_object().ok_or_else(|| {
            DestinationError::Invalid("ObjectStore must be an object".to_owned())
        })?)?;
    }
    let default = endpoint
        .get("default_object_store_id")
        .and_then(Value::as_str);
    if default.is_none() && stores.len() > 1 {
        return Err(DestinationError::Invalid(
            "multiple discovered ObjectStores require an explicit default ObjectStore".to_owned(),
        ));
    }
    if let Some(default) = default
        && !stores
            .iter()
            .any(|store| store.get("object_store_id").and_then(Value::as_str) == Some(default))
    {
        return Err(DestinationError::Invalid(
            "default ObjectStore ID was not discovered".to_owned(),
        ));
    }
    Ok(())
}

fn validate_store(store: &serde_json::Map<String, Value>) -> Result<(), DestinationError> {
    reject_unknown(
        store,
        &[
            "object_store_id",
            "display_name",
            "writable",
            "health",
            "quota_available_bytes",
            "object_types",
        ],
    )?;
    identifier(store, "object_store_id")?;
    nonempty(store, "display_name")?;
    if store.get("writable").and_then(Value::as_bool).is_none() {
        return Err(DestinationError::Invalid(
            "ObjectStore writable must be boolean".to_owned(),
        ));
    }
    one_of(store, "health", &["ready", "read_only", "unavailable"])?;
    number(store, "quota_available_bytes")?;
    let types = store
        .get("object_types")
        .and_then(Value::as_array)
        .filter(|types| !types.is_empty())
        .ok_or_else(|| {
            DestinationError::Invalid("ObjectStore object_types must be non-empty".to_owned())
        })?;
    for value in types {
        match value.as_str() {
            Some("image" | "video" | "bioinformatics") => (),
            _ => {
                return Err(DestinationError::Invalid(
                    "ObjectStore object_types contains an unsupported type".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn reject_unknown(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
) -> Result<(), DestinationError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(DestinationError::Invalid(format!("unknown field `{key}`")));
        }
    }
    Ok(())
}
fn require_exact(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), DestinationError> {
    if object.get(key).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(DestinationError::Invalid(format!(
            "`{key}` must be `{expected}`"
        )))
    }
}
fn nonempty(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, DestinationError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .map(ToOwned::to_owned)
        .ok_or_else(|| DestinationError::Invalid(format!("`{key}` must be a non-empty string")))
}
fn identifier(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, DestinationError> {
    let value = nonempty(object, key)?;
    if value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        Ok(value)
    } else {
        Err(DestinationError::Invalid(format!(
            "`{key}` must be a stable identifier"
        )))
    }
}
fn host_ref(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, DestinationError> {
    let value = nonempty(object, key)?;
    if value.starts_with("monas.host-context:") {
        Ok(value)
    } else {
        Err(DestinationError::Invalid(format!(
            "`{key}` must be an opaque Monas host-context reference"
        )))
    }
}
fn opaque_or_null(
    object: &serde_json::Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<(), DestinationError> {
    match object.get(key).and_then(Value::as_str) {
        Some(value) if value.starts_with(prefix) && value.len() <= 256 => Ok(()),
        _ => Err(DestinationError::Invalid(format!(
            "`{key}` must be an opaque `{prefix}` reference"
        ))),
    }
}
fn require_null(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<(), DestinationError> {
    if object.get(key) == Some(&Value::Null) {
        Ok(())
    } else {
        Err(DestinationError::Invalid(format!("`{key}` must be null")))
    }
}
fn one_of(
    object: &serde_json::Map<String, Value>,
    key: &str,
    values: &[&str],
) -> Result<String, DestinationError> {
    match object.get(key).and_then(Value::as_str) {
        Some(value) if values.contains(&value) => Ok(value.to_owned()),
        _ => Err(DestinationError::Invalid(format!(
            "`{key}` has an unsupported value"
        ))),
    }
}
fn number(object: &serde_json::Map<String, Value>, key: &str) -> Result<u64, DestinationError> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| DestinationError::Invalid(format!("`{key}` must be an unsigned integer")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_destination_fixtures_are_strict_and_fail_closed() {
        let fixture: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/das-destinations/v1/cases.json"
        ))
        .expect("fixture parses");
        assert_eq!(
            fixture["schema_version"],
            "x-img.das-destination-fixtures.v1"
        );
        let cases = fixture["cases"].as_array().expect("cases array");
        validate_reviewed_destination(
            &serde_json::to_vec(&cases[0]["document"]).expect("case JSON"),
        )
        .expect("accepted case");
        for case in &cases[1..] {
            let error = validate_reviewed_destination(
                &serde_json::to_vec(&case["document"]).expect("case JSON"),
            )
            .expect_err("invalid case must fail");
            assert!(
                error
                    .to_string()
                    .contains(case["expected"].as_str().expect("expected message")),
                "case {} returned {error}",
                case["id"]
            );
        }
    }
}
