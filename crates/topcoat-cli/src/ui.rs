mod add;
mod init;
mod list;
mod remove;

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct UiCommand {
    #[command(subcommand)]
    command: UiSubcommand,
}

#[derive(Subcommand)]
enum UiSubcommand {
    /// Set up the package's install state (run before adding components)
    Init(init::InitCommand),
    /// Add a premade UI component to your package
    Add(add::AddCommand),
    /// List registry components and their install status
    List(list::ListCommand),
    /// Remove a previously added UI component from your package
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

/// The `--package` selector shared by the `ui` subcommands: the cargo package
/// to operate on, whose root holds `components.toml`.
#[derive(Args)]
struct PackageArg {
    /// Package to operate on, by name (like `cargo -p`); its root holds
    /// `components.toml` (defaults to the package containing the current
    /// directory)
    #[arg(short, long, value_name = "SPEC")]
    package: Option<String>,
}
