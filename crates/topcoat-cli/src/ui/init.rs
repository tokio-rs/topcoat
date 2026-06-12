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
    /// Location of the default registry (a path, `file://` path, or
    /// `http(s)://` URL); defaults to the built-in `topcoat` registry
    #[arg(long)]
    registry_url: Option<String>,
    #[command(flatten)]
    project: ProjectArg,
}

impl InitCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let options = InitOptions {
            components_dir: self.components_dir,
            registry_url: self.registry_url,
        };
        let initialized = manage::init(&project, options).await?;

        println!(
            "{} initialized {} {}",
            style("+").green(),
            style(initialized.state_file.display()).bold(),
            style(format!(
                "({} registry, components under {})",
                initialized.registry,
                initialized.components_dir.display()
            ))
            .dim(),
        );
        Ok(())
    }
}
