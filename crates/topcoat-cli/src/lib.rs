#![cfg_attr(docsrs, feature(doc_cfg))]
//! The `topcoat` command-line tool.
//!
//! This crate builds the `topcoat` binary, and the `cargo-topcoat` binary
//! that makes the same commands available as `cargo topcoat`. Install it with
//! `cargo install topcoat-cli`, then run `topcoat --help` to see the
//! commands. The library part only exists to share code between the two
//! binaries.

mod asset;
mod common;
mod dev;
mod fmt;
mod ui;

use clap::{Parser, Subcommand};

/// The Topcoat command-line tool.
#[derive(Parser)]
#[command(name = "topcoat")]
pub struct TopcoatCli {
    #[command(subcommand)]
    command: Command,
}

impl TopcoatCli {
    /// Runs the selected command.
    pub async fn run(self) {
        match self.command {
            Command::Ui(cmd) => cmd.run(),
            Command::Fmt(cmd) => cmd.run().await,
            Command::Dev(cmd) => cmd.run().await,
            Command::Asset(cmd) => cmd.run().await,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Start a development server
    Dev(dev::DevCommand),
    /// Format the bodies of Topcoat macros like `view!`
    Fmt(fmt::FmtCommand),
    /// Inspect assets embedded in the binary
    Asset(asset::AssetCommand),
    /// Manage premade UI components in your project
    Ui(ui::UiCommand),
}

/// Parses the command line and runs the selected command.
///
/// Before that, it prints a warning when the project depends on a `topcoat`
/// version that this CLI does not support.
pub async fn run() {
    common::version::warn_on_mismatch();
    TopcoatCli::parse().run().await;
}
