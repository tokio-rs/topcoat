use clap::Args;
use console::style;
use topcoat_ui::manage::{self, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct RemoveCommand {
    /// Names of the components to remove
    #[arg(required = true)]
    components: Vec<String>,
    /// Registry the components were added from (searched across all if omitted)
    #[arg(short, long)]
    registry: Option<String>,
    #[command(flatten)]
    project: ProjectArg,
}

impl RemoveCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let removed = manage::remove(&project, &self.components, self.registry.as_deref())?;

        for component in removed {
            println!(
                "{} removed {} {} from {}",
                style("-").red(),
                style(component.name).bold(),
                style(format!("({})", component.file.display())).dim(),
                component.registry
            );
        }
        Ok(())
    }
}
