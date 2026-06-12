use std::path::PathBuf;

use clap::Args;
use console::style;
use topcoat_ui::manage::{self, InitOptions, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct InitCommand {
    /// Base directory for component install output (defaults to `src/components`)
    #[arg(short, long)]
    components_dir: Option<PathBuf>,
    #[command(flatten)]
    project: ProjectArg,
}

impl InitCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let options = InitOptions {
            components_dir: self.components_dir,
        };
        let initialized = manage::init(&project, options)?;

        println!(
            "{} initialized {} {}",
            style("+").green(),
            style(initialized.state_file.display()).bold(),
            style(format!(
                "(components under {})",
                initialized.components_dir.display()
            ))
            .dim(),
        );
        Ok(())
    }
}
