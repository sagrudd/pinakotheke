// SPDX-License-Identifier: MPL-2.0
//! Shared command implementation for Pinakotheke and its legacy `x-img` alias.

use std::{ffi::OsString, path::PathBuf};

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use x_img_core::{ConfigStore, build_info};

mod launchd;
mod local_objectstore;
mod monolith;
mod object_read_helper;

/// Canonical command name used by the v1 entry point.
pub const CANONICAL_COMMAND: &str = "pinakotheke";
/// Compatibility command retained for pre-v1 scripts.
pub const LEGACY_COMMAND: &str = "x-img";
/// Stable, non-sensitive compatibility notice.
pub const LEGACY_NOTICE: &str =
    "x-img is the legacy command name; migrate to pinakotheke before the v2 compatibility deadline";

/// Identity selected from the invoked executable name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invocation {
    /// The canonical Pinakotheke entry point.
    Canonical,
    /// The legacy x-img compatibility entry point.
    Legacy,
}

impl Invocation {
    /// Resolve only the exact canonical filename as canonical; installed legacy
    /// wrappers and test harnesses remain conservatively compatible.
    #[must_use]
    pub fn from_executable(executable: &std::ffi::OsStr) -> Self {
        let name = std::path::Path::new(executable)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str);
        if name == Some(CANONICAL_COMMAND) {
            Self::Canonical
        } else {
            Self::Legacy
        }
    }

    /// User-visible command name for clap help, version, and diagnostics.
    #[must_use]
    pub const fn command_name(self) -> &'static str {
        match self {
            Self::Canonical => CANONICAL_COMMAND,
            Self::Legacy => LEGACY_COMMAND,
        }
    }

    /// Compatibility warning emitted only by the legacy entry point.
    #[must_use]
    pub const fn notice(self) -> Option<&'static str> {
        match self {
            Self::Canonical => None,
            Self::Legacy => Some(LEGACY_NOTICE),
        }
    }
}

/// Parsed CLI contract shared by both entry points.
#[derive(Debug, PartialEq, Eq, Parser)]
#[command(version, about = "Pinakotheke personal media catalogue")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
enum Command {
    /// Strictly validate and inspect a local versioned configuration file.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Run the local Pinakotheke monolith in the foreground.
    Serve(monolith::ServeArgs),
    /// Provision and inspect monolith storage authorities.
    Storage {
        #[command(subcommand)]
        command: local_objectstore::StorageCommand,
    },
    /// Manage the composed per-user macOS service.
    Service {
        #[command(subcommand)]
        command: launchd::ServiceCommand,
    },
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
enum ConfigCommand {
    /// Validate a configuration file without modifying it.
    Validate(ConfigPath),
    /// List configured account and website rule identifiers without authority references.
    List(ConfigPath),
    /// Validate a complete candidate then atomically replace the destination file.
    Replace(ReplaceConfig),
}

#[derive(Debug, PartialEq, Eq, Args)]
struct ConfigPath {
    /// Local JSON configuration file.
    #[arg(long)]
    path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Args)]
struct ReplaceConfig {
    /// Destination local JSON configuration file.
    #[arg(long)]
    path: PathBuf,
    /// Complete candidate JSON file to validate and atomically install.
    #[arg(long)]
    input: PathBuf,
}

/// Parse arguments using the invoked entry point's visible identity.
pub fn parse_from<I, T>(invocation: Invocation, arguments: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = Cli::command()
        .name(invocation.command_name())
        .bin_name(invocation.command_name())
        .try_get_matches_from(arguments)?;
    Cli::from_arg_matches(&matches)
}

/// Execute an already parsed command.
pub fn run(invocation: Invocation, cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        None => println!(
            "{} {} workspace; no live integrations are enabled.",
            invocation.command_name(),
            build_info().product.version
        ),
        Some(Command::Config { command }) => run_config(command)?,
        Some(Command::Serve(arguments)) => monolith::serve(arguments)?,
        Some(Command::Storage { command }) => local_objectstore::run(command)?,
        Some(Command::Service { command }) => launchd::run(command)?,
    }
    Ok(())
}

fn run_config(command: ConfigCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        ConfigCommand::Validate(arguments) => {
            let config = ConfigStore::new(arguments.path).load()?;
            println!(
                "valid {} configuration {}",
                config.schema_version, config.instance_id
            );
        }
        ConfigCommand::List(arguments) => {
            let config = ConfigStore::new(arguments.path).load()?;
            for account in config.x_accounts {
                println!("x\t{}\t{}", account.account_id, account.handle);
            }
            for account in config.instagram_accounts {
                println!("instagram\t{}\t{}", account.account_id, account.username);
            }
            for policy in config.website_policies {
                println!("website\t{}\t{}", policy.site_id, policy.origin);
            }
        }
        ConfigCommand::Replace(arguments) => {
            let candidate = std::fs::read(arguments.input)?;
            ConfigStore::new(arguments.path).replace_from_json(&candidate)?;
            println!("validated and atomically replaced configuration");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CANONICAL_COMMAND, Invocation, LEGACY_COMMAND, LEGACY_NOTICE, parse_from};

    #[test]
    fn executable_identity_is_exact_and_notice_is_legacy_only() {
        assert_eq!(
            Invocation::from_executable("/usr/bin/pinakotheke".as_ref()),
            Invocation::Canonical
        );
        assert_eq!(Invocation::Canonical.command_name(), CANONICAL_COMMAND);
        assert_eq!(Invocation::Canonical.notice(), None);
        assert_eq!(
            Invocation::from_executable("/usr/bin/x-img".as_ref()),
            Invocation::Legacy
        );
        assert_eq!(Invocation::Legacy.command_name(), LEGACY_COMMAND);
        assert_eq!(Invocation::Legacy.notice(), Some(LEGACY_NOTICE));
    }

    #[test]
    fn canonical_and_legacy_entry_points_parse_identically() {
        let arguments = ["command", "config", "validate", "--path", "fixture.json"];
        let canonical = parse_from(Invocation::Canonical, arguments).expect("canonical arguments");
        let legacy = parse_from(Invocation::Legacy, arguments).expect("legacy arguments");
        assert_eq!(canonical, legacy);
    }
}
