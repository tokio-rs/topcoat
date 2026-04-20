mod dev;
mod fmt;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "topcoat")]
pub struct TopcoatCli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Format topcoat `view!` macros
    Fmt(fmt::FmtCommand),
    /// Start a development server
    Dev(dev::DevCommand),
}

pub async fn run() {
    let cli = TopcoatCli::parse();
    match cli.command {
        Command::Fmt(cmd) => cmd.run().await.unwrap(),
        Command::Dev(cmd) => cmd.run().await,
    }
}
