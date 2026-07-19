// SPDX-License-Identifier: MPL-2.0
//! Guarded metadata catalogue maintenance commands.

use std::{
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Args, Subcommand};
use x_img_core::{
    capture_plan_journal::CapturePlanJournal, gallery_catalogue::GalleryCatalogueStore,
    x_image_reconciliation::reconcile_x_image_catalogue,
};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum CatalogueCommand {
    /// Preview or apply stable X-image identity reconciliation.
    ReconcileXImages(ReconcileXImagesArgs),
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

pub(crate) fn run(command: CatalogueCommand) -> Result<(), Box<dyn std::error::Error>> {
    let CatalogueCommand::ReconcileXImages(arguments) = command;
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
