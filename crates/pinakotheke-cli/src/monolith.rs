// SPDX-License-Identifier: MPL-2.0
//! Foreground, per-user Pinakotheke monolith runner.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use axum_server::tls_rustls::RustlsConfig;
use clap::{Args, Subcommand};
use serde::Deserialize;
use x_img_core::{
    gallery_catalogue::GalleryCatalogueStore,
    site_corpus::SiteCorpusStore,
    viewed_media::{AdapterKind, CapturePairing, CapturePlanService, SiteCapturePolicy},
};

const DEFAULT_PORT: u16 = 8731;

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ServeArgs {
    /// Per-user metadata root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Listener address. Loopback is the safe default.
    #[arg(long, default_value = "127.0.0.1")]
    bind: IpAddr,
    /// Listener port.
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,
    /// PEM certificate chain terminated directly by Axum/Rustls.
    #[arg(long, requires = "tls_private_key")]
    tls_certificate_chain: Option<PathBuf>,
    /// PEM private key paired with --tls-certificate-chain.
    #[arg(long, requires = "tls_certificate_chain")]
    tls_private_key: Option<PathBuf>,
    /// Explicitly acknowledge that a non-loopback listener has no composed authentication yet.
    #[arg(long)]
    allow_non_loopback_without_authentication: bool,
    /// Private file containing the process-local Monas dispatch token.
    #[arg(long)]
    monas_dispatch_token_file: Option<PathBuf>,
    /// Built Trunk output; defaults to ROOT/web, then packaged assets.
    #[arg(long)]
    web_root: Option<PathBuf>,
    /// Directory containing reviewed signed Firefox XPI packages.
    #[arg(long)]
    firefox_downloads_root: Option<PathBuf>,
    /// Absolute executable implementing the scoped object-read helper v1 protocol.
    #[arg(long)]
    object_read_helper: Option<PathBuf>,
    /// Private metadata-only Firefox pairing/site authority document.
    #[arg(long)]
    capture_authority_file: Option<PathBuf>,
    /// Private process token authorizing verified capture-worker completions.
    #[arg(long)]
    capture_completion_token_file: Option<PathBuf>,
    /// Reviewed host executable for continuous approved image acquisition.
    #[arg(long)]
    capture_acquire_helper: Option<PathBuf>,
}

const CAPTURE_AUTHORITY_SCHEMA: &str = "pinakotheke.capture-authority.v1";
const CAPTURE_AUTHORITY_LIMIT: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum CaptureCommand {
    /// Acquire and settle exactly one approved pending image plan.
    Acquire(CaptureAcquireArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct CaptureAcquireArgs {
    /// Product metadata root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Private pairing/site/destination authority document.
    #[arg(long)]
    capture_authority_file: PathBuf,
    /// Absolute reviewed executable implementing capture acquire helper v1.
    #[arg(long)]
    helper: PathBuf,
    /// Authenticated actor identity owning the pending plan.
    #[arg(long)]
    actor_id: String,
    /// Exact pending plan identity.
    #[arg(long)]
    plan_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureAuthorityDocument {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    pairings: Vec<CapturePairingRecord>,
    sites: Vec<SiteCapturePolicyRecord>,
}

#[derive(Debug)]
struct LoadedCaptureAuthority {
    plans: CapturePlanService,
    endpoint_id: String,
    object_store_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturePairingRecord {
    pairing_id: String,
    actor_id: String,
    expires_at: u64,
    revoked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteCapturePolicyRecord {
    site_id: String,
    origin: String,
    capture_enabled: bool,
    adapter_kind: AdapterKind,
    adapter_version: String,
    allow_observed_thumbnails: bool,
    allow_explicit_originals: bool,
    max_candidates_per_page: u64,
}

#[derive(Debug)]
struct LocalRootLayout {
    root: PathBuf,
}

struct CaptureWorkerLease(PathBuf);

impl CaptureWorkerLease {
    fn acquire(root: &Path) -> io::Result<Self> {
        let path = root.join("run/capture-worker.lock");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "another capture worker owns the local lease",
                    )
                } else {
                    error
                }
            })?;
        Ok(Self(path))
    }
}

impl Drop for CaptureWorkerLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl LocalRootLayout {
    fn resolve(requested: Option<PathBuf>) -> io::Result<Self> {
        let root = match requested {
            Some(root) => root,
            None => PathBuf::from(std::env::var_os("HOME").ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "HOME is required when --root is omitted",
                )
            })?)
            .join(".x-img"),
        };
        if !root.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "monolith root must be absolute",
            ));
        }
        if let Ok(metadata) = std::fs::symlink_metadata(&root)
            && (metadata.file_type().is_symlink() || !metadata.is_dir())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "monolith root must be a real directory, not a symlink",
            ));
        }
        std::fs::create_dir_all(&root)?;
        set_private_permissions(&root)?;
        for child in ["config", "state", "run", "logs"] {
            let path = root.join(child);
            if let Ok(metadata) = std::fs::symlink_metadata(&path)
                && (metadata.file_type().is_symlink() || !metadata.is_dir())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} must be a real directory", path.display()),
                ));
            }
            std::fs::create_dir_all(&path)?;
            set_private_permissions(&path)?;
        }
        Ok(Self { root })
    }
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn socket_address(arguments: &ServeArgs) -> io::Result<SocketAddr> {
    if !arguments.bind.is_loopback() && !arguments.allow_non_loopback_without_authentication {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "non-loopback bind refused; use --allow-non-loopback-without-authentication only after reviewing network exposure",
        ));
    }
    Ok(SocketAddr::new(arguments.bind, arguments.port))
}

fn tls_paths(arguments: &ServeArgs) -> io::Result<Option<(&Path, &Path)>> {
    match (
        arguments.tls_certificate_chain.as_deref(),
        arguments.tls_private_key.as_deref(),
    ) {
        (None, None) => Ok(None),
        (Some(certificate), Some(key)) => {
            for (label, path) in [
                ("TLS certificate chain", certificate),
                ("TLS private key", key),
            ] {
                if !path.is_absolute() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{label} path must be absolute"),
                    ));
                }
                let metadata = std::fs::symlink_metadata(path)?;
                if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{label} must be a non-empty regular file"),
                    ));
                }
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if std::fs::metadata(key)?.permissions().mode() & 0o077 != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "TLS private key must not be accessible by group or other users",
                    ));
                }
            }
            Ok(Some((certificate, key)))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "TLS certificate chain and private key must be supplied together",
        )),
    }
}

fn resolve_web_root(
    requested: Option<PathBuf>,
    product_root: &Path,
    installed: Option<&str>,
) -> io::Result<Option<PathBuf>> {
    let candidate = match requested {
        Some(candidate) => candidate,
        None => {
            let candidate = product_root.join("web");
            match std::fs::symlink_metadata(&candidate) {
                Ok(_) => candidate,
                Err(error) if error.kind() == io::ErrorKind::NotFound => match installed {
                    Some(installed) => {
                        let installed = PathBuf::from(installed);
                        match std::fs::symlink_metadata(&installed) {
                            Ok(_) => installed,
                            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                                return Ok(None);
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    None => return Ok(None),
                },
                Err(error) => return Err(error),
            }
        }
    };
    if !candidate.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "web root must be absolute",
        ));
    }
    let root_metadata = std::fs::symlink_metadata(&candidate)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "web root must be a real directory",
        ));
    }
    let mut files = 0;
    let mut bytes = 0;
    validate_web_tree(&candidate, &mut files, &mut bytes)?;
    let index = candidate.join("index.html");
    let metadata = std::fs::symlink_metadata(&index)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "web root requires a regular index.html",
        ));
    }
    Ok(Some(candidate))
}

fn resolve_downloads_root(requested: Option<PathBuf>) -> io::Result<Option<PathBuf>> {
    let Some(path) = requested else {
        return Ok(None);
    };
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Firefox downloads root must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Firefox downloads root must be a real directory",
        ));
    }
    Ok(Some(path))
}

fn validate_web_tree(path: &Path, files: &mut usize, bytes: &mut u64) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "web assets must not contain symlinks",
        ));
    }
    if metadata.is_file() {
        *files += 1;
        *bytes = bytes.saturating_add(metadata.len());
        if *files > 128 || *bytes > 32 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "web asset tree exceeds its bounded size",
            ));
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "web assets must be regular files and directories",
        ));
    }
    for entry in std::fs::read_dir(path)? {
        validate_web_tree(&entry?.path(), files, bytes)?;
    }
    Ok(())
}

pub(crate) fn serve(arguments: ServeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let address = socket_address(&arguments)?;
    let tls = tls_paths(&arguments)?
        .map(|(certificate, key)| (certificate.to_path_buf(), key.to_path_buf()));
    let layout = LocalRootLayout::resolve(arguments.root)?;
    let _lease = CaptureWorkerLease::acquire(&layout.root)?;
    let web_root = resolve_web_root(
        arguments.web_root,
        &layout.root,
        option_env!("PINAKOTHEKE_DEFAULT_WEB_ROOT"),
    )?;
    let downloads_root = resolve_downloads_root(arguments.firefox_downloads_root)?;
    let object_read_backend = arguments
        .object_read_helper
        .as_deref()
        .map(crate::object_read_helper::backend)
        .transpose()?;
    let capture_authority = arguments
        .capture_authority_file
        .as_deref()
        .map(|path| load_capture_authority(path, layout.root.join("state/capture-plans.v1.json")))
        .transpose()?;
    if arguments.capture_completion_token_file.is_some()
        && (object_read_backend.is_none()
            || capture_authority.is_none()
            || arguments.monas_dispatch_token_file.is_none())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "capture completion requires Monas dispatch, capture authority, and object delivery",
        )
        .into());
    }
    let gallery_path = layout.root.join("state/gallery-catalogue.v1.json");
    let gallery_store = GalleryCatalogueStore::new(&gallery_path);
    let capture_completion = arguments
        .capture_completion_token_file
        .as_deref()
        .map(read_dispatch_token)
        .transpose()?
        .map(|token| {
            let authority = capture_authority
                .as_ref()
                .expect("validated capture authority");
            x_img_api::CaptureCompletionAuthority::new(
                token,
                gallery_store.clone(),
                authority.endpoint_id.clone(),
                authority.object_store_id.clone(),
            )
        })
        .transpose()?;
    if arguments.capture_acquire_helper.is_some() && capture_completion.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "continuous capture acquisition requires capture completion authority",
        )
        .into());
    }
    let capture_acquire = arguments
        .capture_acquire_helper
        .as_deref()
        .map(|helper| {
            let authority = capture_authority
                .as_ref()
                .expect("validated capture authority");
            crate::capture_worker_helper::backend(
                helper,
                authority.endpoint_id.clone(),
                authority.object_store_id.clone(),
            )
        })
        .transpose()?;
    let extension_onboarding = capture_authority
        .as_ref()
        .map(|authority| {
            x_img_api::ExtensionOnboardingAuthority::new(
                "pinakotheke-monolith".into(),
                authority.endpoint_id.clone(),
                authority.object_store_id.clone(),
                format!("/downloads/pinakotheke-{}.xpi", env!("CARGO_PKG_VERSION")),
            )
        })
        .transpose()?;
    let capture_plans = capture_authority.map(|authority| authority.plans);
    let monas_dispatch = arguments
        .monas_dispatch_token_file
        .as_deref()
        .map(read_dispatch_token)
        .transpose()?
        .map(x_img_api::MonasDispatchVerifier::new)
        .transpose()?;
    if !address.ip().is_loopback() {
        eprintln!("warning: unauthenticated monolith is exposed beyond loopback at {address}");
    }
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let storage_ready = crate::local_objectstore::is_ready(&layout.root);
        let gallery = gallery_store
            .load_or_empty()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        println!(
            "Pinakotheke {} listening on {}://{address}",
            if tls.is_some() { "https" } else { "http" },
            env!("CARGO_PKG_VERSION")
        );
        println!("metadata root: {}", layout.root.display());
        println!("gallery metadata: {}", gallery_path.display());
        println!(
            "web application: {}",
            web_root
                .as_deref()
                .map_or("Not installed".into(), |path| path.display().to_string())
        );
        println!(
            "object delivery: {}",
            if object_read_backend.is_some() {
                "Host helper configured"
            } else {
                "Not configured"
            }
        );
        println!(
            "Firefox capture planning: {}",
            if capture_plans.is_some() {
                "Monas-authenticated"
            } else {
                "Not configured"
            }
        );
        println!(
            "readiness: {}://{address}/ready",
            if tls.is_some() { "https" } else { "http" }
        );
        let router = match (object_read_backend, capture_plans) {
            (Some(backend), capture_plans) => {
                x_img_api::monolith_router_with_gallery_web_delivery_and_capture_authority(
                    storage_ready,
                    monas_dispatch,
                    gallery,
                    web_root,
                    backend,
                    capture_plans.map(|plans| {
                        let composition =
                            x_img_api::CapturePlanComposition::new(plans, capture_completion);
                        let composition = match capture_acquire {
                            Some(backend) => composition.with_acquire(backend),
                            None => composition,
                        };
                        match extension_onboarding {
                            Some(onboarding) => composition
                                .with_onboarding(onboarding)
                                .with_site_corpus(SiteCorpusStore::new(
                                    layout.root.join("state/site-corpus.v1.json"),
                                )),
                            None => composition,
                        }
                    }),
                )
            }
            (None, Some(capture_plans)) => {
                x_img_api::monolith_router_with_gallery_web_and_capture_authority(
                    storage_ready,
                    monas_dispatch,
                    gallery,
                    web_root,
                    capture_plans,
                )
            }
            (None, None) => x_img_api::monolith_router_with_gallery_and_web_authority(
                storage_ready,
                monas_dispatch,
                gallery,
                web_root,
            ),
        };
        let router = match downloads_root {
            Some(root) => x_img_api::with_firefox_downloads(router, root),
            None => router,
        };
        if let Some((certificate, key)) = tls {
            let config = RustlsConfig::from_pem_file(certificate, key).await?;
            axum_server::bind_rustls(address, config)
                .serve(router.into_make_service())
                .await
        } else {
            let listener = tokio::net::TcpListener::bind(address).await?;
            axum::serve(listener, router).await
        }
    })?;
    Ok(())
}

pub(crate) fn run_capture(command: CaptureCommand) -> Result<(), Box<dyn std::error::Error>> {
    let CaptureCommand::Acquire(arguments) = command;
    if !safe_identifier(&arguments.actor_id) || !safe_identifier(&arguments.plan_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "capture actor and plan identities are invalid",
        )
        .into());
    }
    let layout = LocalRootLayout::resolve(arguments.root)?;
    let _lease = CaptureWorkerLease::acquire(&layout.root)?;
    let mut authority = load_capture_authority(
        &arguments.capture_authority_file,
        layout.root.join("state/capture-plans.v1.json"),
    )?;
    let plan = authority
        .plans
        .pending(&arguments.actor_id, &arguments.plan_id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "pending capture plan not found"))?;
    let evidence = crate::capture_worker_helper::acquire(
        &arguments.helper,
        &plan,
        &authority.endpoint_id,
        &authority.object_store_id,
    )?;
    let outcome = x_img_core::capture_completion::complete_verified_image(
        &mut authority.plans,
        GalleryCatalogueStore::new(layout.root.join("state/gallery-catalogue.v1.json")),
        &arguments.actor_id,
        evidence,
    )?;
    println!("capture {} settled: {outcome:?}", arguments.plan_id);
    Ok(())
}

fn load_capture_authority(
    path: &Path,
    journal_path: PathBuf,
) -> io::Result<LoadedCaptureAuthority> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > CAPTURE_AUTHORITY_LIMIT
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "capture authority must be an absolute bounded regular file",
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
    let bytes = std::fs::read(path)?;
    let document: CaptureAuthorityDocument = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if document.schema_version != CAPTURE_AUTHORITY_SCHEMA
        || !safe_identifier(&document.endpoint_id)
        || !safe_identifier(&document.object_store_id)
        || document.pairings.len() > 128
        || document.sites.len() > 256
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture authority schema or record count is invalid",
        ));
    }
    let mut pairing_ids = std::collections::BTreeSet::new();
    let pairings = document
        .pairings
        .into_iter()
        .map(|record| {
            if !safe_identifier(&record.pairing_id)
                || !safe_identifier(&record.actor_id)
                || record.expires_at == 0
                || !pairing_ids.insert(record.pairing_id.clone())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "capture pairing record is invalid or duplicated",
                ));
            }
            Ok(CapturePairing {
                pairing_id: record.pairing_id,
                actor_id: record.actor_id,
                expires_at: record.expires_at,
                revoked: record.revoked,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let mut origins = std::collections::BTreeSet::new();
    let sites = document
        .sites
        .into_iter()
        .map(|record| {
            if !safe_identifier(&record.site_id)
                || !safe_origin(&record.origin)
                || !safe_semver(&record.adapter_version)
                || record.max_candidates_per_page == 0
                || record.max_candidates_per_page > 1_000
                || !origins.insert(record.origin.clone())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "capture site policy is invalid or duplicated",
                ));
            }
            Ok(SiteCapturePolicy {
                site_id: record.site_id,
                origin: record.origin,
                capture_enabled: record.capture_enabled,
                adapter_kind: record.adapter_kind,
                adapter_version: record.adapter_version,
                allow_observed_thumbnails: record.allow_observed_thumbnails,
                allow_explicit_originals: record.allow_explicit_originals,
                max_candidates_per_page: record.max_candidates_per_page,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let plans = CapturePlanService::with_journal(pairings, sites, journal_path)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(LoadedCaptureAuthority {
        plans,
        endpoint_id: document.endpoint_id,
        object_store_id: document.object_store_id,
    })
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn safe_origin(value: &str) -> bool {
    value
        .strip_prefix("https://")
        .is_some_and(|host| !host.is_empty() && !host.contains(['/', '?', '#', '@', '*']))
        && value.len() <= 512
}

fn safe_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn read_dispatch_token(path: &Path) -> io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Monas dispatch token must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Monas dispatch token file must not be accessible by group or others",
            ));
        }
    }
    let token = std::fs::read_to_string(path)?;
    Ok(token.trim_end_matches(['\r', '\n']).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "pinakotheke-monolith-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn creates_only_private_metadata_directories() {
        let root = temporary_root();
        let layout = LocalRootLayout::resolve(Some(root.clone())).unwrap();
        for child in ["config", "state", "run", "logs"] {
            assert!(layout.root.join(child).is_dir());
        }
        assert!(!layout.root.join("dasobjectstore").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_non_loopback_without_the_explicit_acknowledgement() {
        let denied = ServeArgs {
            root: None,
            bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: DEFAULT_PORT,
            tls_certificate_chain: None,
            tls_private_key: None,
            allow_non_loopback_without_authentication: false,
            monas_dispatch_token_file: None,
            web_root: None,
            firefox_downloads_root: None,
            object_read_helper: None,
            capture_authority_file: None,
            capture_completion_token_file: None,
            capture_acquire_helper: None,
        };
        assert_eq!(
            socket_address(&denied).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        let reviewed = ServeArgs {
            allow_non_loopback_without_authentication: true,
            ..denied
        };
        assert_eq!(
            socket_address(&reviewed).unwrap(),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT))
        );
    }

    #[test]
    fn tls_assets_are_paired_absolute_regular_and_private() {
        let root = temporary_root();
        std::fs::create_dir_all(&root).unwrap();
        let certificate = root.join("server.crt");
        let key = root.join("server.key");
        std::fs::write(&certificate, "synthetic certificate").unwrap();
        std::fs::write(&key, "synthetic private key").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mut arguments = ServeArgs {
            root: None,
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_PORT,
            tls_certificate_chain: Some(certificate.clone()),
            tls_private_key: Some(key.clone()),
            allow_non_loopback_without_authentication: false,
            monas_dispatch_token_file: None,
            web_root: None,
            firefox_downloads_root: None,
            object_read_helper: None,
            capture_authority_file: None,
            capture_completion_token_file: None,
            capture_acquire_helper: None,
        };
        assert_eq!(
            tls_paths(&arguments).unwrap(),
            Some((certificate.as_path(), key.as_path()))
        );

        arguments.tls_private_key = None;
        assert_eq!(
            tls_paths(&arguments).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        arguments.tls_private_key = Some(key.clone());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o640)).unwrap();
            assert_eq!(
                tls_paths(&arguments).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dispatch_token_requires_a_private_regular_file() {
        let root = temporary_root();
        std::fs::create_dir_all(&root).unwrap();
        let token = root.join("dispatch.token");
        std::fs::write(&token, "synthetic-dispatch-token-00000001\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(
            read_dispatch_token(&token).unwrap(),
            "synthetic-dispatch-token-00000001"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(
                read_dispatch_token(&token).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn capture_authority_is_private_strict_and_bounded() {
        let root = temporary_root();
        std::fs::create_dir_all(&root).unwrap();
        let authority = root.join("capture-authority.json");
        std::fs::write(
            &authority,
            r#"{
              "schema_version":"pinakotheke.capture-authority.v1",
              "endpoint_id":"endpoint-1",
              "object_store_id":"store-1",
              "pairings":[{"pairing_id":"pair-1","actor_id":"actor-1","expires_at":4102444800,"revoked":false}],
              "sites":[{"site_id":"example","origin":"https://example.invalid","capture_enabled":true,"adapter_kind":"experimental_generic","adapter_version":"1.0.0","allow_observed_thumbnails":true,"allow_explicit_originals":true,"max_candidates_per_page":32}]
            }"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&authority, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let journal = root.join("capture-plans.json");
        assert!(load_capture_authority(&authority, journal.clone()).is_ok());
        std::fs::write(
            &authority,
            r#"{"schema_version":"future.v2","pairings":[],"sites":[]}"#,
        )
        .unwrap();
        assert_eq!(
            load_capture_authority(&authority, journal)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn run_one_capture_drives_helper_to_persistent_gallery_settlement() {
        use std::os::unix::fs::PermissionsExt;
        use x_img_core::viewed_media::{
            AdapterKind, CAPTURE_REQUEST_SCHEMA_VERSION, CaptureKind, CapturePlanRequest,
        };

        let root = temporary_root();
        LocalRootLayout::resolve(Some(root.clone())).unwrap();
        let authority_path = root.join("config/capture-authority.json");
        std::fs::write(
            &authority_path,
            r#"{
              "schema_version":"pinakotheke.capture-authority.v1",
              "endpoint_id":"endpoint-1",
              "object_store_id":"store-1",
              "pairings":[{"pairing_id":"pair-1","actor_id":"actor-1","expires_at":4102444800,"revoked":false}],
              "sites":[{"site_id":"example","origin":"https://example.invalid","capture_enabled":true,"adapter_kind":"experimental_generic","adapter_version":"1.0.0","allow_observed_thumbnails":true,"allow_explicit_originals":true,"max_candidates_per_page":32}]
            }"#,
        )
        .unwrap();
        std::fs::set_permissions(&authority_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let journal = root.join("state/capture-plans.v1.json");
        let mut loaded = load_capture_authority(&authority_path, journal).unwrap();
        let plan = loaded
            .plans
            .plan(
                "actor-1",
                1,
                CapturePlanRequest {
                    schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
                    pairing_id: "pair-1".into(),
                    origin: "https://example.invalid".into(),
                    page_url: "https://example.invalid/gallery".into(),
                    adapter_kind: AdapterKind::ExperimentalGeneric,
                    adapter_version: "1.0.0".into(),
                    capture_kind: CaptureKind::ObservedThumbnail,
                    media_url: "https://example.invalid/thumb.jpg".into(),
                    presentation_url: None,
                    width: 320,
                    height: 200,
                },
            )
            .unwrap();
        drop(loaded);
        let helper = root.join("helper.sh");
        std::fs::write(
            &helper,
            r##"#!/bin/sh
read request
printf '%s\n' '{"outcome":"committed","schema_version":"pinakotheke.capture-acquire-helper.v1","catalogue_id":"card-1","title":"Synthetic image","content_type":"image/jpeg","content_length":42,"endpoint_id":"endpoint-1","object_store_id":"store-1","object_key":"object-1","object_version":2,"checksum_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","verified_at_epoch_seconds":42}' >&2
"##,
        )
        .unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        run_capture(CaptureCommand::Acquire(CaptureAcquireArgs {
            root: Some(root.clone()),
            capture_authority_file: authority_path,
            helper,
            actor_id: "actor-1".into(),
            plan_id: plan.plan_id,
        }))
        .unwrap();
        assert_eq!(
            GalleryCatalogueStore::new(root.join("state/gallery-catalogue.v1.json"))
                .load_or_empty()
                .unwrap()
                .items()
                .len(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn web_root_requires_a_bounded_regular_trunk_tree() {
        let root = temporary_root();
        let web = root.join("web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html>").unwrap();
        std::fs::write(web.join("pinakotheke.js"), "export {};").unwrap();
        assert_eq!(
            resolve_web_root(None, &root, Some("/not/used")).unwrap(),
            Some(web.clone())
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(web.join("pinakotheke.js"), web.join("linked.js")).unwrap();
            assert_eq!(
                resolve_web_root(Some(web.clone()), &root, None)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_web_root_is_a_validated_fallback() {
        let root = temporary_root();
        let installed = temporary_root().join("installed-web");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("index.html"), "<!doctype html>").unwrap();
        assert_eq!(
            resolve_web_root(None, &root, installed.to_str()).unwrap(),
            Some(installed.clone())
        );

        std::fs::write(root.join("web"), "not a directory").unwrap();
        assert_eq!(
            resolve_web_root(None, &root, installed.to_str())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(installed.parent().unwrap()).unwrap();
    }
}
