// SPDX-License-Identifier: MPL-2.0
//! Command-line entry point for the x-img scaffold.

use clap::Parser;
use x_img_core::build_info;

/// x-img command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "x-img",
    version,
    about = "x-img media catalogue workspace scaffold"
)]
struct Cli;

fn main() {
    let _cli = Cli::parse();
    println!(
        "{} workspace scaffold; no live integrations are enabled.",
        build_info().summary()
    );
}
