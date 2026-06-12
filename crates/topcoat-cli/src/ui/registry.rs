use clap::{Args, Subcommand};
use console::style;
use topcoat_ui::manage::{self, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct RegistryCommand {
    #[command(subcommand)]
    command: RegistrySubcommand,
}

#[derive(Subcommand)]
enum RegistrySubcommand {
    /// Add a registry to the project's install state
    Add(AddCommand),
    /// Remove a registry from the project's install state
    Remove(RemoveCommand),
}

#[derive(Args)]
struct AddCommand {
    /// Registry location (a path, `file://` path, or `http(s)://` URL)
    url: String,
    /// Name to track the registry under (defaults to the name it declares)
    name: Option<String>,
    #[command(flatten)]
    project: ProjectArg,
}

#[derive(Args)]
struct RemoveCommand {
    /// Name of the registry to remove
    name: String,
    #[command(flatten)]
    project: ProjectArg,
}

impl RegistryCommand {
    pub(super) async fn run(self) {
        let result = match self.command {
            RegistrySubcommand::Add(cmd) => cmd.run().await,
            RegistrySubcommand::Remove(cmd) => cmd.run(),
        };
        if let Err(error) = result {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }
}

impl AddCommand {
    async fn run(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let added = manage::add_registry(&project, &self.url, self.name.as_deref()).await?;

        println!(
            "{} added registry {} {}",
            style("+").green(),
            style(&added.name).bold(),
            style(format!(
                "({}, components under {})",
                added.url,
                added.components_dir.display()
            ))
            .dim(),
        );
        Ok(())
    }
}

impl RemoveCommand {
    fn run(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let removed = manage::remove_registry(&project, &self.name)?;

        println!(
            "{} removed registry {} {}",
            style("-").red(),
            style(&removed.name).bold(),
            style(format!("({})", removed.url)).dim(),
        );
        Ok(())
    }
}
