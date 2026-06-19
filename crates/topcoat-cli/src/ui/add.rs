use clap::Args;
use console::style;
use topcoat_ui::manage::{self, AddOptions, AddOutcome, Package};

use super::PackageArg;

#[derive(Args)]
pub(super) struct AddCommand {
    /// Names of the components to add (e.g. `button card`)
    #[arg(required = true)]
    components: Vec<String>,
    /// Registry crate to add from (defaults to the built-in default registry)
    #[arg(short, long)]
    registry: Option<String>,
    /// Overwrite the component file if it already exists
    #[arg(short, long)]
    overwrite: bool,
    #[command(flatten)]
    package: PackageArg,
}

impl AddCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let package = Package::locate(self.package.package)?;
        let options = AddOptions {
            components: self.components,
            registry: self.registry,
            overwrite: self.overwrite,
        };

        let mut confirm = confirm;
        match manage::add(&package, &options, &mut confirm)? {
            AddOutcome::UpToDate => {
                println!("{} already up to date", style("✓").green());
            }
            AddOutcome::Added(added) => {
                for component in added {
                    println!(
                        "{} added {} {} from {}",
                        style("+").green(),
                        style(component.name).bold(),
                        style(format!("({})", component.file.display())).dim(),
                        component.registry
                    );
                }
            }
        }
        Ok(())
    }
}

/// Asks the user a yes/no question on the terminal, defaulting to no. Errors
/// when there is no terminal to prompt on, so non-interactive use must be
/// explicit (via `--registry`).
fn confirm(prompt: &str) -> Result<bool, String> {
    use std::io::{IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "{prompt} (no terminal to prompt on; pass --registry to choose)"
        ));
    }

    eprint!("{} {} ", style(prompt).yellow(), style("[y/N]").dim());
    std::io::stderr().flush().ok();

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|error| format!("failed to read input: {error}"))?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
