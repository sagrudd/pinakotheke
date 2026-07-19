// SPDX-License-Identifier: MPL-2.0
//! Application-core boundaries with no live source or storage integration.

pub mod account_refresh;
pub mod acquisition;
pub mod application_identity;
pub mod bioinformatics;
pub mod bioinformatics_commit;
pub mod cache_alias;
pub mod capture_completion;
pub mod capture_plan_journal;
pub mod compliance_reconciliation;
pub mod connector_fixtures;
pub mod destination;
pub mod destination_selection;
pub mod extension_pairing;
pub mod gallery_catalogue;
pub mod host_context;
pub mod host_product;
pub mod instagram_discovery;
pub mod migration_backup;
pub mod object_ingest;
pub mod object_read;
pub mod operations;
pub mod persistent_gallery_admission;
pub mod persistent_video_gallery;
pub mod playback_delivery;
pub mod reconciliation;
pub mod review_admission;
pub mod reviewed_destination;
pub mod scheduler;
pub mod segmented_video;
pub mod site_corpus;
pub mod synoptikon_catalogue;
pub mod video_candidate;
pub mod video_normalization;
pub mod video_profile;
pub mod viewed_media;
pub mod website_capture_review;
pub mod x_discovery;
pub mod x_followed_accounts;
pub mod x_oauth;

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use x_img_model::{ConfigValidationError, InstanceConfig, ProductIdentity, product_identity};

/// Build information exposed uniformly to the CLI, API host, and web client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// Product identity for this build.
    pub product: ProductIdentity,
}

impl BuildInfo {
    /// Returns a concise human-readable summary.
    #[must_use]
    pub fn summary(self) -> String {
        format!("{} {}", self.product.name, self.product.version)
    }
}

/// Returns the current build information.
#[must_use]
pub const fn build_info() -> BuildInfo {
    BuildInfo {
        product: product_identity(),
    }
}

/// Error returned when a local configuration cannot be read, validated, or saved.
#[derive(Debug)]
pub enum ConfigStoreError {
    /// The filesystem rejected a configuration operation.
    Io(io::Error),
    /// The configuration was not valid JSON or did not match the strict model.
    Json(serde_json::Error),
    /// The configuration parsed but violated a policy invariant.
    Validation(ConfigValidationError),
}

impl std::fmt::Display for ConfigStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "configuration I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "configuration JSON is invalid: {error}"),
            Self::Validation(error) => {
                write!(formatter, "configuration validation failed: {error}")
            }
        }
    }
}

impl std::error::Error for ConfigStoreError {}

impl From<io::Error> for ConfigStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ConfigStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<ConfigValidationError> for ConfigStoreError {
    fn from(error: ConfigValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Local owner of a single versioned configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// Creates a store rooted at an explicit configuration path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the configured path without reading or writing it.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads and strictly validates the configuration at the configured path.
    pub fn load(&self) -> Result<InstanceConfig, ConfigStoreError> {
        Self::parse(&fs::read(&self.path)?)
    }

    /// Parses and strictly validates a complete configuration document.
    pub fn parse(bytes: &[u8]) -> Result<InstanceConfig, ConfigStoreError> {
        let config = serde_json::from_slice::<InstanceConfig>(bytes)?;
        config.validate()?;
        Ok(config)
    }

    /// Validates and atomically replaces the complete configuration document.
    ///
    /// The temporary file is created beside the destination, synchronised, and
    /// renamed only after validation succeeds. The destination's parent must
    /// already exist; this avoids silently creating an unintended config root.
    pub fn replace(&self, config: &InstanceConfig) -> Result<(), ConfigStoreError> {
        config.validate()?;
        let encoded = serde_json::to_vec_pretty(config)?;
        self.write_atomic(&encoded)
    }

    /// Parses, validates, and atomically replaces using a JSON candidate.
    pub fn replace_from_json(&self, bytes: &[u8]) -> Result<(), ConfigStoreError> {
        let config = Self::parse(bytes)?;
        self.replace(&config)
    }

    fn write_atomic(&self, bytes: &[u8]) -> Result<(), ConfigStoreError> {
        static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuration path must have a parent directory",
            )
        })?;
        let filename = self.path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuration path must name a file",
            )
        })?;
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            filename.to_string_lossy(),
            std::process::id(),
            TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> Result<(), ConfigStoreError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &self.path)?;
            sync_directory(parent)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn sync_directory(path: &Path) -> Result<(), io::Error> {
    #[cfg(unix)]
    {
        fs::File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::Value;

    use super::{ConfigStore, ConfigStoreError, build_info};

    fn valid_config() -> Value {
        serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/config/instance.v1.json"
        )))
        .expect("synthetic fixture is valid JSON")
    }

    fn parse(value: Value) -> Result<(), ConfigStoreError> {
        ConfigStore::parse(&serde_json::to_vec(&value).expect("fixture serializes")).map(|_| ())
    }

    fn temporary_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "x-img-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time after epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn summary_contains_the_workspace_version() {
        assert_eq!(build_info().summary(), "Pinakotheke 1.22.3");
    }

    #[test]
    fn accepts_the_complete_synthetic_fixture() {
        assert!(parse(valid_config()).is_ok());
    }

    #[test]
    fn rejects_unknown_properties_and_future_schema_majors() {
        let unknown = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/config/invalid-unknown-field.v1.json"
        ));
        let future = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/config/invalid-future-major.json"
        ));
        assert!(ConfigStore::parse(unknown).is_err());
        assert!(ConfigStore::parse(future).is_err());
    }

    #[test]
    fn rejects_duplicate_accounts_and_origins() {
        let mut accounts = valid_config();
        let x_accounts = accounts["x_accounts"]
            .as_array_mut()
            .expect("accounts array");
        let mut duplicate_handle = x_accounts[0].clone();
        duplicate_handle["account_id"] = Value::String("different-x-account".into());
        x_accounts.push(duplicate_handle);
        assert!(parse(accounts).is_err());

        let mut origins = valid_config();
        let policies = origins["website_policies"]
            .as_array_mut()
            .expect("sites array");
        let mut duplicate_origin = policies[0].clone();
        duplicate_origin["site_id"] = Value::String("different-site".into());
        policies.push(duplicate_origin);
        assert!(parse(origins).is_err());
    }

    #[test]
    fn rejects_invalid_handles_and_wildcard_origins() {
        let mut handle = valid_config();
        handle["x_accounts"][0]["handle"] = Value::String("@invalid".into());
        assert!(parse(handle).is_err());

        let mut wildcard = valid_config();
        wildcard["website_policies"][0]["origin"] =
            Value::String("https://*.example.invalid".into());
        assert!(parse(wildcard).is_err());
    }

    #[test]
    fn rejects_enabled_accounts_without_an_opaque_authorization_reference() {
        let mut account = valid_config();
        account["x_accounts"][0]
            .as_object_mut()
            .expect("account object")
            .remove("authorization_ref");
        assert!(parse(account).is_err());
    }

    #[test]
    fn replaces_only_a_valid_complete_document() {
        let directory = temporary_path("config-store");
        fs::create_dir(&directory).expect("temporary directory");
        let path = directory.join("instance.json");
        let store = ConfigStore::new(&path);
        let encoded = serde_json::to_vec(&valid_config()).expect("fixture serializes");
        store
            .replace_from_json(&encoded)
            .expect("valid config writes");
        assert_eq!(
            store.load().expect("config loads").instance_id,
            "fixture-instance"
        );

        let invalid = br#"{"schema_version":"x-img.instance.v9"}"#;
        assert!(store.replace_from_json(invalid).is_err());
        assert_eq!(
            store.load().expect("existing config remains").instance_id,
            "fixture-instance"
        );
        fs::remove_dir_all(directory).expect("temporary directory removed");
    }
}
