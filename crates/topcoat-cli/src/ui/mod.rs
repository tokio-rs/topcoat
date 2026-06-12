mod add;
mod init;
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
    /// Set up the project's install state (run before adding components)
    Init(init::InitCommand),
    /// Add a premade UI component to your project
    Add(add::AddCommand),
    /// List registry components and their install status
    List(list::ListCommand),
    /// Remove a previously added UI component from your project
    Remove(remove::RemoveCommand),
}

impl UiCommand {
    pub fn run(self) {
        match self.command {
            UiSubcommand::Init(cmd) => cmd.run(),
            UiSubcommand::Add(cmd) => cmd.run(),
            UiSubcommand::List(cmd) => cmd.run(),
            UiSubcommand::Remove(cmd) => cmd.run(),
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
