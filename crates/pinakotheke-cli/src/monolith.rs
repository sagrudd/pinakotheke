// SPDX-License-Identifier: MPL-2.0
//! Foreground, per-user Pinakotheke monolith runner.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use clap::Args;
use x_img_core::gallery_catalogue::GalleryCatalogueStore;

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
    /// Explicitly acknowledge that a non-loopback listener has no composed authentication yet.
    #[arg(long)]
    allow_non_loopback_without_authentication: bool,
    /// Private file containing the process-local Monas dispatch token.
    #[arg(long)]
    monas_dispatch_token_file: Option<PathBuf>,
    /// Built Trunk output; defaults to ROOT/web, then packaged assets.
    #[arg(long)]
    web_root: Option<PathBuf>,
    /// Absolute executable implementing the scoped object-read helper v1 protocol.
    #[arg(long)]
    object_read_helper: Option<PathBuf>,
}

#[derive(Debug)]
struct LocalRootLayout {
    root: PathBuf,
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
    let layout = LocalRootLayout::resolve(arguments.root)?;
    let web_root = resolve_web_root(
        arguments.web_root,
        &layout.root,
        option_env!("PINAKOTHEKE_DEFAULT_WEB_ROOT"),
    )?;
    let object_read_backend = arguments
        .object_read_helper
        .as_deref()
        .map(crate::object_read_helper::backend)
        .transpose()?;
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
        let listener = tokio::net::TcpListener::bind(address).await?;
        let storage_ready = crate::local_objectstore::is_ready(&layout.root);
        let gallery_path = layout.root.join("state/gallery-catalogue.v1.json");
        let gallery = GalleryCatalogueStore::new(&gallery_path)
            .load_or_empty()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        println!(
            "Pinakotheke {} listening on http://{address}",
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
        println!("readiness: http://{address}/ready");
        match object_read_backend {
            Some(backend) => {
                x_img_api::serve_monolith_with_gallery_web_and_delivery(
                    listener,
                    storage_ready,
                    monas_dispatch,
                    gallery,
                    web_root,
                    backend,
                )
                .await
            }
            None => {
                x_img_api::serve_monolith_with_gallery_and_web(
                    listener,
                    storage_ready,
                    monas_dispatch,
                    gallery,
                    web_root,
                )
                .await
            }
        }
    })?;
    Ok(())
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
            allow_non_loopback_without_authentication: false,
            monas_dispatch_token_file: None,
            web_root: None,
            object_read_helper: None,
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
