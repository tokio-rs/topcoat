use clap::Args;
use console::style;
use topcoat_ui::manage::{self, Package};

use super::PackageArg;

#[derive(Args)]
pub(super) struct RemoveCommand {
    /// Names of the components to remove
    #[arg(required = true)]
    components: Vec<String>,
    /// Registry the components were added from (searched across all if omitted)
    #[arg(short, long)]
    registry: Option<String>,
    #[command(flatten)]
    package: PackageArg,
}

impl RemoveCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let package = Package::locate(self.package.package)?;
        let removed = manage::remove(&package, &self.components, self.registry.as_deref())?;

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
