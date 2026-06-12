use std::path::PathBuf;

use clap::Args;
use console::style;
use topcoat_ui::manage::{self, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct InitCommand {
    /// Base directory for component install output (defaults to `src/components`)
    #[arg(short, long)]
    base_dir: Option<PathBuf>,
    #[command(flatten)]
    project: ProjectArg,
}

impl InitCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let initialized = manage::init(&project, self.base_dir)?;

        println!(
            "{} initialized {} {}",
            style("+").green(),
            style(initialized.state_file.display()).bold(),
            style(format!("(components install under {})", initialized.base_dir.display())).dim(),
        );
        Ok(())
    }
}
