use clap::Args;
use console::style;
use topcoat_ui::manage::{self, InstallStatus, Project};

use super::ProjectArg;

#[derive(Args)]
pub(super) struct ListCommand {
    /// Limit the listing to a single named registry
    #[arg(short, long)]
    registry: Option<String>,
    /// Only show components that are installed
    #[arg(short, long)]
    installed: bool,
    #[command(flatten)]
    project: ProjectArg,
}

impl ListCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project.project)?;
        let listings = manage::list(&project, self.registry.as_deref()).await?;

        for listing in &listings {
            // Separate registry blocks with a blank line.
            println!();
            println!(
                "{} {}",
                style(&listing.name).bold(),
                style(format!("({})", listing.url)).dim()
            );

            let components = match &listing.outcome {
                Ok(components) => components,
                Err(error) => {
                    println!(
                        "  {}",
                        style(format!("failed to load registry: {error}")).red()
                    );
                    continue;
                }
            };

            for component in components {
                match &component.status {
                    InstallStatus::Available { .. } => {
                        if !self.installed {
                            println!("    {}", component.name);
                        }
                    }
                    InstallStatus::UpToDate { .. } => {
                        println!(
                            "  {} {} {}",
                            style("✓").green(),
                            style(&component.name).bold(),
                            style("(installed)").dim()
                        );
                    }
                    InstallStatus::Update { .. } => {
                        println!(
                            "  {} {} {}",
                            style("↑").yellow(),
                            style(&component.name).bold(),
                            style("(update available)").yellow(),
                        );
                    }
                    InstallStatus::Orphaned { .. } => {
                        println!(
                            "  {} {} {}",
                            style("?").red(),
                            component.name,
                            style("(not in registry)").red(),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
