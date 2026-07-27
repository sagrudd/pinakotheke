// SPDX-License-Identifier: MPL-2.0
//! Guarded metadata catalogue maintenance commands.

use std::{
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Args, Subcommand};
use serde::Deserialize;
use x_img_core::{
    capture_plan_journal::CapturePlanJournal, gallery_catalogue::GalleryCatalogueStore,
    gallery_reconciliation::reconcile_gallery, x_image_reconciliation::reconcile_x_image_catalogue,
};

const CAPTURE_AUTHORITY_SCHEMA: &str = "pinakotheke.capture-authority.v1";

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum CatalogueCommand {
    /// Preview or apply stable X-image identity reconciliation.
    ReconcileXImages(ReconcileXImagesArgs),
    /// Compare the gallery projection with one reviewed DAS authority inventory.
    ReconcileAuthority(ReconcileAuthorityArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ReconcileXImagesArgs {
    /// Product metadata root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Replace both metadata documents after private backups.
    #[arg(long, requires = "confirm_service_stopped")]
    apply: bool,
    /// Confirm that Pinakotheke and its capture worker are stopped.
    #[arg(long, requires = "apply")]
    confirm_service_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ReconcileAuthorityArgs {
    /// Product metadata root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Private reviewed endpoint/ObjectStore authority document.
    #[arg(long)]
    capture_authority_file: PathBuf,
    /// Absolute executable implementing gallery-inventory helper v2.
    #[arg(long)]
    helper: PathBuf,
    /// Persist only availability repairs after service shutdown confirmation.
    #[arg(long, requires = "confirm_service_stopped")]
    apply: bool,
    /// Confirm that Pinakotheke and its capture worker are stopped.
    #[arg(long, requires = "apply")]
    confirm_service_stopped: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureAuthorityDocument {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    pairings: Vec<CapturePairingRecord>,
    #[serde(default, rename = "sites")]
    _sites: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturePairingRecord {
    pairing_id: String,
    actor_id: String,
    expires_at: u64,
    revoked: bool,
}

pub(crate) fn run(command: CatalogueCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CatalogueCommand::ReconcileXImages(arguments) => reconcile_x_images(arguments),
        CatalogueCommand::ReconcileAuthority(arguments) => reconcile_authority(arguments),
    }
}

fn reconcile_x_images(arguments: ReconcileXImagesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let root = arguments.root.map_or_else(default_root, Ok)?;
    let state = root.join("state");
    let gallery = GalleryCatalogueStore::new(state.join("gallery-catalogue.v1.json"));
    let journal = CapturePlanJournal::new(state.join("capture-plans.v1.json"));
    let original_items = gallery.load_or_empty()?.items().to_vec();
    let original_plans = journal.load()?;
    let result = reconcile_x_image_catalogue(original_items.clone(), original_plans.clone())?;

    println!(
        "X image reconciliation: {} duplicate group(s), {} redundant card(s), {} card identity rename(s), {} plan binding update(s), {} source link(s) added",
        result.report.duplicate_groups,
        result.report.redundant_cards,
        result.report.renamed_cards,
        result.report.rebound_plans,
        result.report.source_links_added,
    );
    if !arguments.apply {
        println!("Dry run only; no metadata was changed.");
        return Ok(());
    }
    if root.join("run/capture-worker.lock").exists() {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "capture worker is active; stop Pinakotheke before applying reconciliation",
        )
        .into());
    }
    if !result.report.changed() {
        println!("Catalogue is already reconciled.");
        return Ok(());
    }

    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    backup(gallery.path(), suffix)?;
    backup(journal.path(), suffix)?;
    journal.replace(&result.plans)?;
    if let Err(error) = gallery.replace(result.items) {
        let rollback = journal.replace(&original_plans);
        return match rollback {
            Ok(()) => Err(error.into()),
            Err(rollback) => Err(io::Error::other(format!(
                "gallery reconciliation failed and journal rollback failed: {error}; {rollback}"
            ))
            .into()),
        };
    }
    println!("Reconciled metadata successfully; DASObjectStore objects were not deleted.");
    Ok(())
}

fn reconcile_authority(
    arguments: ReconcileAuthorityArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = arguments.root.map_or_else(default_root, Ok)?;
    let state = root.join("state");
    let gallery = GalleryCatalogueStore::new(state.join("gallery-catalogue.v1.json"));
    let authority = load_authority_destination(&arguments.capture_authority_file)?;
    let inventory = crate::gallery_inventory_helper::backend(
        &arguments.helper,
        authority.endpoint_id,
        authority.object_store_id,
    )?;
    let objects = inventory().map_err(io::Error::other)?;
    let original = gallery.load_or_empty()?;
    let mut candidate = original.clone();
    let report = reconcile_gallery(&mut candidate, &objects);
    println!(
        "Authority reconciliation: {} protected, {} projected, {} orphan, {} stale, {} changed representation(s)",
        report.authoritative_count,
        report.projected_count,
        report.orphan_count,
        report.stale_count,
        report.changed_representations,
    );
    if !arguments.apply {
        println!("Dry run only; no metadata or DASObjectStore object was changed.");
        return Ok(());
    }
    if root.join("run/capture-worker.lock").exists() {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "capture worker is active; stop Pinakotheke before applying reconciliation",
        )
        .into());
    }
    if report.changed_representations == 0 {
        println!("Gallery projection is already reconciled.");
        return Ok(());
    }
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    backup(gallery.path(), suffix)?;
    gallery.replace(candidate.items().to_vec())?;
    println!(
        "Reconciled gallery availability successfully; DASObjectStore objects were not created, deleted, or altered."
    );
    Ok(())
}

fn load_authority_destination(path: &std::path::Path) -> io::Result<CaptureAuthorityDocument> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "capture authority file must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "capture authority file must not be accessible by group or others",
            ));
        }
    }
    let bytes = fs::read(path)?;
    if bytes.is_empty() || bytes.len() > 256 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture authority document has an invalid size",
        ));
    }
    let authority: CaptureAuthorityDocument = serde_json::from_slice(&bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "capture authority document is invalid",
        )
    })?;
    if authority.schema_version != CAPTURE_AUTHORITY_SCHEMA
        || authority.endpoint_id.is_empty()
        || authority.endpoint_id.len() > 128
        || authority.object_store_id.is_empty()
        || authority.object_store_id.len() > 128
        || authority.pairings.is_empty()
        || authority.pairings.len() > 128
        || authority.pairings.iter().any(|pairing| {
            !safe_identifier(&pairing.pairing_id)
                || !safe_identifier(&pairing.actor_id)
                || pairing.expires_at == 0
                || pairing.revoked
        })
        || authority
            .pairings
            .iter()
            .map(|pairing| &pairing.pairing_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != authority.pairings.len()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture authority does not define a reviewed destination",
        ));
    }
    Ok(authority)
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn default_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not configured"))?;
    Ok(PathBuf::from(home).join(".x-img"))
}

fn backup(path: &std::path::Path, suffix: u64) -> io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid metadata filename"))?;
    let backup = path.with_file_name(format!("{file_name}.pre-x-image-reconcile-{suffix}.bak"));
    let options = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)?;
    drop(options);
    if let Err(error) = fs::copy(path, &backup) {
        let _ = fs::remove_file(&backup);
        return Err(error);
    }
    Ok(())
}
