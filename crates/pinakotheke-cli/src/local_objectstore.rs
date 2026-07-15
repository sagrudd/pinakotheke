// SPDX-License-Identifier: MPL-2.0
//! DASObjectStore-owned local profile orchestration for the monolith.

use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

const PROFILE_ID: &str = "pinakotheke-local";
const STORE_ID: &str = "pinakotheke_local";
const STORE_BUCKET: &str = "pinakotheke-local";
const STORE_PREFIX: &str = "media";
const CONSUMER: &str = "pinakotheke";
const DESCRIPTION_SCHEMA: &str = "dasobjectstore.local_profile_description.v1";
const SELECTION_SCHEMA: &str = "pinakotheke.local-objectstore-selection.v1";

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub(crate) enum StorageCommand {
    /// Manage the DASObjectStore-owned local profile.
    LocalProfile {
        #[command(subcommand)]
        command: LocalProfileCommand,
    },
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub(crate) enum LocalProfileCommand {
    /// Print the reviewed non-secret profile plan without changing anything.
    Plan(ProfileArgs),
    /// Ask DASObjectStore to provision, then record its secret-free identity.
    Provision(ProfileArgs),
    /// Re-discover and verify the provisioned authority identity.
    Status(ProfileArgs),
    /// Ask DASObjectStore to stop services without deleting state.
    Down(ProfileArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ProfileArgs {
    /// Pinakotheke product root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
    /// DASObjectStore's canonical local-profile authority helper.
    #[arg(long)]
    provisioner: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalProfilePlan {
    product_root: PathBuf,
    storage_root: PathBuf,
    private_root: PathBuf,
    selection_path: PathBuf,
    provisioner: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityDescription {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    profile_id: String,
    status: String,
    api_url: String,
    credential_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalSelection<'a> {
    schema_version: &'static str,
    endpoint_id: &'a str,
    object_store_id: &'a str,
    profile_id: &'a str,
    api_url: &'a str,
    credential_ref: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedLocalSelection {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    profile_id: String,
    api_url: String,
    credential_ref: String,
}

pub(crate) fn is_ready(product_root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(product_root.join("state/dasobjectstore-local.json")) else {
        return false;
    };
    let Ok(selection) = serde_json::from_slice::<OwnedLocalSelection>(&bytes) else {
        return false;
    };
    selection.schema_version == SELECTION_SCHEMA
        && selection.object_store_id == STORE_ID
        && selection.profile_id == PROFILE_ID
        && selection.endpoint_id.starts_with("local-docker-")
        && selection.api_url == "http://127.0.0.1:3900"
        && selection
            .credential_ref
            .starts_with("dasobjectstore.local-profile:pinakotheke-local:")
}

pub(crate) fn run(command: StorageCommand) -> Result<(), Box<dyn std::error::Error>> {
    let (action, arguments) = match command {
        StorageCommand::LocalProfile {
            command: LocalProfileCommand::Plan(args),
        } => ("plan", args),
        StorageCommand::LocalProfile {
            command: LocalProfileCommand::Provision(args),
        } => ("provision", args),
        StorageCommand::LocalProfile {
            command: LocalProfileCommand::Status(args),
        } => ("status", args),
        StorageCommand::LocalProfile {
            command: LocalProfileCommand::Down(args),
        } => ("down", args),
    };
    let plan = LocalProfilePlan::resolve(arguments)?;
    match action {
        "plan" => print_plan(&plan),
        "provision" => {
            authority_command(&plan, "up")?;
            let description = discover(&plan)?;
            persist_selection(&plan, &description)?;
            print_description(&description);
        }
        "status" => print_description(&discover(&plan)?),
        "down" => {
            authority_command(&plan, "down")?;
            println!("DASObjectStore local profile stopped; state retained");
        }
        _ => unreachable!(),
    }
    Ok(())
}

impl LocalProfilePlan {
    fn resolve(arguments: ProfileArgs) -> io::Result<Self> {
        let home = PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is required"))?,
        );
        let product_root = arguments.root.unwrap_or_else(|| home.join(".x-img"));
        if !product_root.is_absolute() || !arguments.provisioner.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "root and provisioner must be absolute",
            ));
        }
        if !arguments.provisioner.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "DASObjectStore provisioner is unavailable",
            ));
        }
        Ok(Self {
            storage_root: product_root.join("dasobjectstore"),
            selection_path: product_root.join("state/dasobjectstore-local.json"),
            private_root: home.join(".config/dasobjectstore"),
            product_root,
            provisioner: arguments.provisioner,
        })
    }
}

fn configured_command(plan: &LocalProfilePlan, action: &str) -> Command {
    let mut command = Command::new(&plan.provisioner);
    command
        .arg(action)
        .env("DASOBJECTSTORE_LOCAL_ROOT", &plan.storage_root)
        .env("DASOBJECTSTORE_LOCAL_PRIVATE_ROOT", &plan.private_root)
        .env("DASOBJECTSTORE_LOCAL_PROFILE", PROFILE_ID)
        .env("DASOBJECTSTORE_LOCAL_STORE_ID", STORE_ID)
        .env("DASOBJECTSTORE_LOCAL_STORE_BUCKET", STORE_BUCKET)
        .env("DASOBJECTSTORE_LOCAL_STORE_PREFIX", STORE_PREFIX)
        .env("DASOBJECTSTORE_LOCAL_CONSUMER", CONSUMER);
    command
}

fn authority_command(plan: &LocalProfilePlan, action: &str) -> io::Result<Output> {
    let output = configured_command(plan, action).output()?;
    if output.status.success() {
        return Ok(output);
    }
    let diagnostic = bounded_authority_diagnostic(&output.stderr);
    Err(io::Error::other(format!(
        "DASObjectStore authority action `{action}` failed with status {}{diagnostic}",
        output.status,
    )))
}

fn bounded_authority_diagnostic(stderr: &[u8]) -> String {
    const LIMIT: usize = 512;
    let text = String::from_utf8_lossy(stderr);
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.is_empty() {
        return String::new();
    }
    let bounded: String = line.chars().take(LIMIT).collect();
    format!(": {bounded}")
}

fn discover(plan: &LocalProfilePlan) -> Result<AuthorityDescription, Box<dyn std::error::Error>> {
    let output = authority_command(plan, "describe")?;
    let description: AuthorityDescription = serde_json::from_slice(&output.stdout)?;
    validate_description(&description)?;
    Ok(description)
}

fn validate_description(description: &AuthorityDescription) -> io::Result<()> {
    if description.schema_version != DESCRIPTION_SCHEMA
        || description.object_store_id != STORE_ID
        || description.profile_id != PROFILE_ID
        || description.status != "ready"
        || !description.endpoint_id.starts_with("local-docker-")
        || description.api_url != "http://127.0.0.1:3900"
        || !description
            .credential_ref
            .starts_with("dasobjectstore.local-profile:pinakotheke-local:")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DASObjectStore returned an unexpected local profile identity",
        ));
    }
    Ok(())
}

fn persist_selection(
    plan: &LocalProfilePlan,
    description: &AuthorityDescription,
) -> io::Result<()> {
    let parent = plan.selection_path.parent().expect("selection has parent");
    std::fs::create_dir_all(parent)?;
    let temporary = plan.selection_path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&LocalSelection {
        schema_version: SELECTION_SCHEMA,
        endpoint_id: &description.endpoint_id,
        object_store_id: &description.object_store_id,
        profile_id: &description.profile_id,
        api_url: &description.api_url,
        credential_ref: &description.credential_ref,
    })?;
    std::fs::write(&temporary, bytes)?;
    set_private_file(&temporary)?;
    std::fs::rename(temporary, &plan.selection_path)
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}
#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn print_plan(plan: &LocalProfilePlan) {
    println!("DASObjectStore managed local profile plan");
    println!("storage root: {}", plan.storage_root.display());
    println!("private authority root: {}", plan.private_root.display());
    println!("profile: {PROFILE_ID}");
    println!("ObjectStore: {STORE_ID}");
    println!("Pinakotheke never writes media directly to the storage root");
}

fn print_description(description: &AuthorityDescription) {
    println!("endpoint: {} (Ready)", description.endpoint_id);
    println!("ObjectStore: {} (Ready)", description.object_store_id);
    println!("profile: {}", description.profile_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pinakotheke-local-profile-{label}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn accepts_only_the_exact_ready_authority_identity() {
        let valid = AuthorityDescription {
            schema_version: DESCRIPTION_SCHEMA.into(),
            endpoint_id: "local-docker-42".into(),
            object_store_id: STORE_ID.into(),
            profile_id: PROFILE_ID.into(),
            status: "ready".into(),
            api_url: "http://127.0.0.1:3900".into(),
            credential_ref: "dasobjectstore.local-profile:pinakotheke-local:42".into(),
        };
        assert!(validate_description(&valid).is_ok());
        let changed = AuthorityDescription {
            object_store_id: "first-store".into(),
            ..valid
        };
        assert_eq!(
            validate_description(&changed).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn readiness_requires_an_exact_safe_persisted_selection() {
        let root = temporary_root("ready");
        let state = root.join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("dasobjectstore-local.json"),
            r#"{
              "schema_version":"pinakotheke.local-objectstore-selection.v1",
              "endpoint_id":"local-docker-a1b2c3",
              "object_store_id":"pinakotheke_local",
              "profile_id":"pinakotheke-local",
              "api_url":"http://127.0.0.1:3900",
              "credential_ref":"dasobjectstore.local-profile:pinakotheke-local:a1b2c3"
            }"#,
        )
        .unwrap();
        assert!(is_ready(&root));

        std::fs::write(
            state.join("dasobjectstore-local.json"),
            r#"{"schema_version":"pinakotheke.local-objectstore-selection.v1"}"#,
        )
        .unwrap();
        assert!(!is_ready(&root));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn authority_diagnostics_are_single_line_and_bounded() {
        let long = format!("ignored\nerror: {}", "x".repeat(700));
        let diagnostic = bounded_authority_diagnostic(long.as_bytes());
        assert!(diagnostic.starts_with(": error: "));
        assert!(diagnostic.len() <= 514);
        assert!(!diagnostic.contains('\n'));
    }
}
