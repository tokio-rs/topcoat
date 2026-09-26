use clap::Args;
use console::style;
use topcoat_ui::manage::{self, AddAction, AddOptions, Package, Selection};

use super::PackageArg;

#[derive(Args)]
pub(super) struct AddCommand {
    /// Names of the components to add (e.g. `button card`)
    #[arg(required_unless_present = "all", conflicts_with = "all")]
    components: Vec<String>,
    /// Add every component the registry offers, skipping those already
    /// installed (combine with `--overwrite` to replace them too)
    #[arg(short, long)]
    all: bool,
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
            selection: if self.all {
                Selection::All
            } else {
                Selection::Named(self.components)
            },
            registry: self.registry,
            overwrite: self.overwrite,
        };

        let mut confirm = confirm;
        let entries = manage::add(&package, &options, &mut confirm)?;

        let mut written = false;
        for entry in entries {
            let file = style(format!("({})", entry.file.display())).dim();
            let name = style(entry.name).bold();
            match entry.action {
                AddAction::Installed => {
                    written = true;
                    println!(
                        "{} added {name} {file} from {}",
                        style("+").green(),
                        entry.registry
                    );
                }
                AddAction::Overwritten => {
                    written = true;
                    println!(
                        "{} updated {name} {file} from {}",
                        style("~").yellow(),
                        entry.registry
                    );
                }
                AddAction::Skipped => {
                    println!(
                        "{}",
                        style(format!("- skipped {name} {file}: already installed")).dim()
                    );
                }
            }
        }
        if !written {
            println!("{} already up to date", style("✓").green());
        }
        Ok(())
    }
}

/// Asks for confirmation, defaulting to no. Returns an error without an interactive
/// terminal. Use an explicit `--registry` to avoid registry-selection prompts.
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
