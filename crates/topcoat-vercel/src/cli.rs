mod build;
mod common;
mod init;

use anyhow::Result;
use build::BuildCommand;
use clap::{Parser, Subcommand};
use init::InitCommand;

#[derive(Parser)]
#[command(name = "topcoat-vercel", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate the Vercel project configuration
    Init(InitCommand),
    /// Build a Vercel Build Output API deployment
    Build(BuildCommand),
}

pub fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Init(command) => command.run(),
        Command::Build(command) => command.run(),
    }
}
