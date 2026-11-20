use clap::Args;
use console::style;
use topcoat_ui::manage::{self, InstallStatus, Package};

use super::PackageArg;

#[derive(Args)]
pub(super) struct ListCommand {
    /// Limit the listing to a single named registry
    #[arg(short, long)]
    registry: Option<String>,
    /// Only show components that are installed
    #[arg(short, long)]
    installed: bool,
    #[command(flatten)]
    package: PackageArg,
}

impl ListCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let package = Package::locate(self.package.package)?;
        let listings = manage::list(&package, self.registry.as_deref())?;

        for listing in &listings {
            // Skip registries that have nothing installed from them.
            if self.installed
                && listing.theme.is_none()
                && listing.outcome.as_ref().is_ok_and(|components| {
                    components.iter().all(|component| {
                        matches!(component.status, InstallStatus::Available { .. })
                    })
                })
            {
                continue;
            }

            // Separate registry blocks with a blank line.
            println!();
            println!("{}", style(&listing.name).bold());

            if let Some(theme) = &listing.theme {
                print_installed("theme ", &theme.name, &theme.status);
            }

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
                if matches!(component.status, InstallStatus::Available { .. }) {
                    if !self.installed {
                        println!("    {}", component.name);
                    }
                } else {
                    print_installed("", &component.name, &component.status);
                }
            }
        }

        Ok(())
    }
}

/// Prints the line for an installed component or theme. `kind` goes before the name,
/// to tell a theme from a component.
fn print_installed(kind: &str, name: &str, status: &InstallStatus) {
    match status {
        InstallStatus::Available { .. } => {}
        InstallStatus::UpToDate { .. } => {
            println!(
                "  {} {kind}{} {}",
                style("✓").green(),
                style(name).bold(),
                style("(installed)").dim()
            );
        }
        InstallStatus::Update { .. } => {
            println!(
                "  {} {kind}{} {}",
                style("↑").yellow(),
                style(name).bold(),
                style("(update available)").yellow(),
            );
        }
        InstallStatus::Orphaned { .. } => {
            println!(
                "  {} {kind}{name} {}",
                style("?").red(),
                style("(orphaned)").red(),
            );
        }
    }
}
