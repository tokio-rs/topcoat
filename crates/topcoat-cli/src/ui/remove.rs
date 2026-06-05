use clap::Args;
use console::style;
use topcoat_ui::manage::{self, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct RemoveCommand {
    /// Name of the component to remove
    component: String,
    /// Registry the component was added from (searched across all if omitted)
    #[arg(short, long)]
    registry: Option<String>,
    #[command(flatten)]
    project: ProjectArg,
}

impl RemoveCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let removed = manage::remove(&project, &self.component, self.registry.as_deref())?;

        println!(
            "{} removed {} {} from {}",
            style("-").red(),
            style(removed.name).bold(),
            style(format!("({})", removed.file.display())).dim(),
            removed.registry
        );
        Ok(())
    }
}
