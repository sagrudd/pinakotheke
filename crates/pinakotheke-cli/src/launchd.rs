// SPDX-License-Identifier: MPL-2.0
//! Non-root launchd management for the composed local service.

use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};

use clap::{Args, Subcommand};

const BACKEND_LABEL: &str = "org.mnemosyne.pinakotheke.backend";
const MONAS_LABEL: &str = "org.mnemosyne.pinakotheke.monas";

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub(crate) enum ServiceCommand {
    /// Review the exact paths and labels without modifying the system.
    Plan(ServiceArgs),
    /// Install and start both per-user agents.
    Install(InstallArgs),
    /// Print launchd status for both agents.
    Status(ServiceArgs),
    /// Restart both agents through launchd.
    Restart(ServiceArgs),
    /// Print log paths, or follow both logs.
    Logs(LogsArgs),
    /// Stop and remove agents while preserving all application data.
    Uninstall(ServiceArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ServiceArgs {
    /// Product metadata root; defaults to $HOME/.x-img.
    #[arg(long)]
    root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct InstallArgs {
    #[command(flatten)]
    service: ServiceArgs,
    /// Absolute path to the Pinakotheke executable.
    #[arg(long)]
    pinakotheke_binary: PathBuf,
    /// Absolute path to the Monas server executable.
    #[arg(long)]
    monas_binary: PathBuf,
    /// Optional absolute scoped ObjectStore read-helper executable.
    #[arg(long)]
    object_read_helper: Option<PathBuf>,
    /// Replace existing Pinakotheke-managed plist definitions.
    #[arg(long)]
    replace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct LogsArgs {
    #[command(flatten)]
    service: ServiceArgs,
    /// Follow all service logs with /usr/bin/tail.
    #[arg(long)]
    follow: bool,
}

#[derive(Debug)]
struct ServiceLayout {
    root: PathBuf,
    launch_agents: PathBuf,
    backend_plist: PathBuf,
    monas_plist: PathBuf,
    token: PathBuf,
    prosopikon_root: PathBuf,
    logs: PathBuf,
}

pub(crate) fn run(command: ServiceCommand) -> Result<(), Box<dyn std::error::Error>> {
    let service = match &command {
        ServiceCommand::Plan(args)
        | ServiceCommand::Status(args)
        | ServiceCommand::Restart(args)
        | ServiceCommand::Uninstall(args) => args,
        ServiceCommand::Install(args) => &args.service,
        ServiceCommand::Logs(args) => &args.service,
    };
    let layout = ServiceLayout::resolve(service.root.clone())?;
    match command {
        ServiceCommand::Plan(_) => print_plan(&layout),
        ServiceCommand::Install(args) => install(&layout, &args)?,
        ServiceCommand::Status(_) => {
            launchctl(&["print", &format!("{}/{}", domain()?, BACKEND_LABEL)])?;
            launchctl(&["print", &format!("{}/{}", domain()?, MONAS_LABEL)])?;
        }
        ServiceCommand::Restart(_) => {
            launchctl(&[
                "kickstart",
                "-k",
                &format!("{}/{}", domain()?, BACKEND_LABEL),
            ])?;
            launchctl(&["kickstart", "-k", &format!("{}/{}", domain()?, MONAS_LABEL)])?;
        }
        ServiceCommand::Logs(args) => logs(&layout, args.follow)?,
        ServiceCommand::Uninstall(_) => uninstall(&layout)?,
    }
    Ok(())
}

impl ServiceLayout {
    fn resolve(requested: Option<PathBuf>) -> io::Result<Self> {
        let home = PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is required"))?,
        );
        let root = requested.unwrap_or_else(|| home.join(".x-img"));
        if !root.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "service root must be absolute",
            ));
        }
        let launch_agents = home.join("Library/LaunchAgents");
        Ok(Self {
            backend_plist: launch_agents.join(format!("{BACKEND_LABEL}.plist")),
            monas_plist: launch_agents.join(format!("{MONAS_LABEL}.plist")),
            token: root.join("run/monas-dispatch.token"),
            prosopikon_root: home.join(".config/monas/prosopikon"),
            logs: root.join("logs"),
            root,
            launch_agents,
        })
    }
}

fn install(layout: &ServiceLayout, args: &InstallArgs) -> io::Result<()> {
    require_macos()?;
    validate_binary(&args.pinakotheke_binary)?;
    validate_binary(&args.monas_binary)?;
    if let Some(helper) = &args.object_read_helper {
        validate_binary(helper)?;
    }
    if !args.replace && (layout.backend_plist.exists() || layout.monas_plist.exists()) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "a Pinakotheke service definition exists; review and use --replace",
        ));
    }
    for directory in [
        &layout.root,
        layout.token.parent().expect("token parent"),
        &layout.prosopikon_root,
        &layout.logs,
        &layout.launch_agents,
    ] {
        std::fs::create_dir_all(directory)?;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    ensure_token(&layout.token)?;
    let backend = backend_plist(
        layout,
        &args.pinakotheke_binary,
        args.object_read_helper.as_deref(),
    );
    let monas = monas_plist(layout, &args.monas_binary);
    let previous_backend = std::fs::read(&layout.backend_plist).ok();
    let previous_monas = std::fs::read(&layout.monas_plist).ok();
    install_plist(&layout.backend_plist, &backend, args.replace)?;
    if let Err(error) = install_plist(&layout.monas_plist, &monas, args.replace) {
        restore_plist(&layout.backend_plist, previous_backend.as_deref())?;
        return Err(error);
    }
    let domain = domain()?;
    if args.replace {
        for label in [MONAS_LABEL, BACKEND_LABEL] {
            let _ = launchctl(&["bootout", &format!("{domain}/{label}")]);
        }
    }
    if let Err(error) = launchctl(&["bootstrap", &domain, path_text(&layout.backend_plist)?]) {
        restore_pair(
            layout,
            previous_backend.as_deref(),
            previous_monas.as_deref(),
            &domain,
        );
        return Err(error);
    }
    if let Err(error) = launchctl(&["bootstrap", &domain, path_text(&layout.monas_plist)?]) {
        let _ = launchctl(&["bootout", &format!("{domain}/{BACKEND_LABEL}")]);
        restore_pair(
            layout,
            previous_backend.as_deref(),
            previous_monas.as_deref(),
            &domain,
        );
        return Err(error);
    }
    println!("Pinakotheke and Monas per-user agents installed");
    Ok(())
}

fn restore_pair(
    layout: &ServiceLayout,
    backend: Option<&[u8]>,
    monas: Option<&[u8]>,
    domain: &str,
) {
    let _ = restore_plist(&layout.backend_plist, backend);
    let _ = restore_plist(&layout.monas_plist, monas);
    if backend.is_some() {
        let _ = launchctl(&[
            "bootstrap",
            domain,
            path_text_unchecked(&layout.backend_plist),
        ]);
    }
    if monas.is_some() {
        let _ = launchctl(&[
            "bootstrap",
            domain,
            path_text_unchecked(&layout.monas_plist),
        ]);
    }
}

fn restore_plist(path: &Path, previous: Option<&[u8]>) -> io::Result<()> {
    match previous {
        Some(contents) => {
            let temporary = path.with_extension("plist.rollback");
            std::fs::write(&temporary, contents)?;
            std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
            std::fs::rename(temporary, path)
        }
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    }
}

fn uninstall(layout: &ServiceLayout) -> io::Result<()> {
    require_macos()?;
    let domain = domain()?;
    for label in [MONAS_LABEL, BACKEND_LABEL] {
        let _ = launchctl(&["bootout", &format!("{domain}/{label}")]);
    }
    for plist in [&layout.monas_plist, &layout.backend_plist] {
        match std::fs::remove_file(plist) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    println!(
        "Service definitions removed; configuration, credentials, logs, catalogue state, and ObjectStore data were preserved"
    );
    Ok(())
}

fn logs(layout: &ServiceLayout, follow: bool) -> io::Result<()> {
    let paths = [
        layout.logs.join("pinakotheke.stdout.log"),
        layout.logs.join("pinakotheke.stderr.log"),
        layout.logs.join("monas.stdout.log"),
        layout.logs.join("monas.stderr.log"),
    ];
    if !follow {
        for path in paths {
            println!("{}", path.display());
        }
        return Ok(());
    }
    let status = Command::new("/usr/bin/tail")
        .arg("-F")
        .args(paths)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("tail failed"))
    }
}

fn ensure_token(path: &Path) -> io::Result<()> {
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "existing dispatch token is not a private regular file",
            ));
        }
        return Ok(());
    }
    let mut bytes = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let token = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    writeln!(file, "{token}")?;
    file.sync_all()
}

fn install_plist(path: &Path, contents: &str, replace: bool) -> io::Result<()> {
    if path.exists() && !replace {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists; review and use --replace", path.display()),
        ));
    }
    let temporary = path.with_extension("plist.tmp");
    std::fs::write(&temporary, contents)?;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(temporary, path)
}

fn backend_plist(layout: &ServiceLayout, binary: &Path, helper: Option<&Path>) -> String {
    let mut arguments = vec![
        "serve",
        "--root",
        path_text_unchecked(&layout.root),
        "--bind",
        "127.0.0.1",
        "--port",
        "8732",
        "--monas-dispatch-token-file",
        path_text_unchecked(&layout.token),
    ];
    if let Some(helper) = helper {
        arguments.extend(["--object-read-helper", path_text_unchecked(helper)]);
    }
    plist(
        BACKEND_LABEL,
        binary,
        &arguments,
        &[],
        layout,
        "pinakotheke",
    )
}

fn monas_plist(layout: &ServiceLayout, binary: &Path) -> String {
    plist(
        MONAS_LABEL,
        binary,
        &[],
        &[
            ("MONAS_BIND_ADDR", "127.0.0.1:8731"),
            ("PINAKOTHEKE_UPSTREAM", "http://127.0.0.1:8732"),
            (
                "PINAKOTHEKE_DISPATCH_TOKEN_FILE",
                path_text_unchecked(&layout.token),
            ),
            (
                "PROSOPIKON_ROOT",
                path_text_unchecked(&layout.prosopikon_root),
            ),
        ],
        layout,
        "monas",
    )
}

fn plist(
    label: &str,
    binary: &Path,
    arguments: &[&str],
    environment: &[(&str, &str)],
    layout: &ServiceLayout,
    log: &str,
) -> String {
    let args = std::iter::once(path_text_unchecked(binary))
        .chain(arguments.iter().copied())
        .map(|value| format!("<string>{}</string>", xml(value)))
        .collect::<Vec<_>>()
        .join("");
    let env = environment
        .iter()
        .map(|(key, value)| format!("<key>{}</key><string>{}</string>", xml(key), xml(value)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>Label</key><string>{}</string><key>ProgramArguments</key><array>{}</array><key>EnvironmentVariables</key><dict>{}</dict><key>RunAtLoad</key><true/><key>KeepAlive</key><true/><key>ProcessType</key><string>Background</string><key>StandardOutPath</key><string>{}</string><key>StandardErrorPath</key><string>{}</string></dict></plist>
"#,
        xml(label),
        args,
        env,
        xml(path_text_unchecked(
            &layout.logs.join(format!("{log}.stdout.log"))
        )),
        xml(path_text_unchecked(
            &layout.logs.join(format!("{log}.stderr.log"))
        ))
    )
}

fn validate_binary(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path).ok();
    if !path.is_absolute()
        || metadata
            .as_ref()
            .is_none_or(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
        || metadata.is_none_or(|metadata| metadata.permissions().mode() & 0o111 == 0)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "service binaries must be absolute executable regular files",
        ));
    }
    Ok(())
}

fn domain() -> io::Result<String> {
    let output = Command::new("/usr/bin/id").arg("-u").output()?;
    if !output.status.success() {
        return Err(io::Error::other("could not determine user id"));
    }
    Ok(format!(
        "gui/{}",
        String::from_utf8_lossy(&output.stdout).trim()
    ))
}

fn launchctl(arguments: &[&str]) -> io::Result<()> {
    require_macos()?;
    let status = Command::new("/bin/launchctl").args(arguments).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "launchctl {} failed",
            arguments.first().unwrap_or(&"command")
        )))
    }
}

fn require_macos() -> io::Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "launchd service management is available only on macOS",
        ))
    }
}

fn path_text(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "paths must be UTF-8"))
}
fn path_text_unchecked(path: &Path) -> &str {
    path.to_str().expect("validated local path is UTF-8")
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn print_plan(layout: &ServiceLayout) {
    println!("Pinakotheke per-user launchd plan");
    println!("backend: {BACKEND_LABEL} (127.0.0.1:8732)");
    println!("host: {MONAS_LABEL} (127.0.0.1:8731)");
    println!("product root: {}", layout.root.display());
    println!(
        "Prosopikon authority root: {}",
        layout.prosopikon_root.display()
    );
    println!("uninstall preserves all data and credentials");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plists_escape_paths_and_separate_host_from_backend() {
        let home = PathBuf::from("/Users/synthetic & user");
        let layout = ServiceLayout {
            root: home.join(".x-img"),
            launch_agents: home.join("Library/LaunchAgents"),
            backend_plist: home.join("backend.plist"),
            monas_plist: home.join("monas.plist"),
            token: home.join(".x-img/run/token"),
            prosopikon_root: home.join(".config/monas/prosopikon"),
            logs: home.join(".x-img/logs"),
        };
        let backend = backend_plist(
            &layout,
            Path::new("/opt/bin/pinakotheke"),
            Some(Path::new("/opt/bin/das-reader")),
        );
        let monas = monas_plist(&layout, Path::new("/opt/bin/monas-server"));
        assert!(backend.contains("127.0.0.1</string><string>--port</string><string>8732"));
        assert!(backend.contains("--object-read-helper"));
        assert!(backend.contains("/opt/bin/das-reader"));
        assert!(monas.contains("127.0.0.1:8731"));
        assert!(monas.contains("PROSOPIKON_ROOT"));
        assert!(monas.contains("synthetic &amp; user"));
        assert!(!monas.contains("monas_session"));
    }
}
