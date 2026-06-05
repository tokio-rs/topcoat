mod add;
mod list;
mod remove;

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct UiCommand {
    #[command(subcommand)]
    command: UiSubcommand,
}

#[derive(Subcommand)]
enum UiSubcommand {
    /// Add a premade UI component to your project
    Add(add::AddCommand),
    /// List registry components and their install status
    List(list::ListCommand),
    /// Remove a previously added UI component from your project
    Remove(remove::RemoveCommand),
}

impl UiCommand {
    pub async fn run(self) {
        match self.command {
            UiSubcommand::Add(cmd) => cmd.run().await,
            UiSubcommand::List(cmd) => cmd.run().await,
            UiSubcommand::Remove(cmd) => cmd.run().await,
        }
    }
}

/// The `--project` selector shared by the `ui` subcommands: the cargo workspace
/// to operate on, whose root holds `components.toml`.
#[derive(Args)]
struct ProjectArg {
    /// Cargo workspace to operate on; its root holds `components.toml`
    /// (defaults to the current workspace)
    #[arg(short, long)]
    project: Option<PathBuf>,
}
