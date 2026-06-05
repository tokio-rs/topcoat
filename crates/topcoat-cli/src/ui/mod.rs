mod add;
mod list;
mod module;
mod project;
mod remove;
mod state;

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
