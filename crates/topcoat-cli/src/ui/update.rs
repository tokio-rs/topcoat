use clap::Args;
use console::style;
use topcoat_ui::manage::{self, Package, UpdateAction, UpdateKind, UpdateOptions};

use super::PackageArg;

#[derive(Args)]
pub(super) struct UpdateCommand {
    /// Names of the components to update (defaults to the theme and every
    /// installed component)
    components: Vec<String>,
    /// Update the theme, alone or along with the named components
    #[arg(short, long)]
    theme: bool,
    /// Show what would happen without writing any file
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    package: PackageArg,
}

impl UpdateCommand {
    pub(super) fn run(self) {
        match self.run_inner() {
            // Conflicts were written and still need resolving.
            Ok(true) => std::process::exit(1),
            Ok(false) => {}
            Err(error) => {
                eprintln!("{}", style(error).red());
                std::process::exit(1);
            }
        }
    }

    /// Runs the update and reports it. Returns whether conflicts were written.
    fn run_inner(self) -> Result<bool, String> {
        let package = Package::locate(self.package.package)?;
        let options = UpdateOptions {
            components: self.components,
            theme: self.theme,
            dry_run: self.dry_run,
        };
        let entries = manage::update(&package, &options)?;

        // In a dry run, each line says what would happen instead.
        let would = if self.dry_run { "would be " } else { "" };
        let mut changed = false;
        let mut conflicts = 0;
        let mut versions = 0;
        for entry in entries {
            let name = match entry.kind {
                UpdateKind::Theme => format!("theme {}", style(entry.name).bold()),
                UpdateKind::Component => style(entry.name).bold().to_string(),
            };
            let file = style(format!("({})", entry.file.display())).dim();
            match entry.action {
                UpdateAction::UpToDate => continue,
                UpdateAction::VersionRecorded => {
                    versions += 1;
                    continue;
                }
                UpdateAction::FastForwarded | UpdateAction::Matched => println!(
                    "{} {name} {file} {would}updated from {}",
                    style("~").yellow(),
                    entry.registry
                ),
                UpdateAction::Merged => println!(
                    "{} {name} {file} {would}merged with your changes from {}",
                    style("~").yellow(),
                    entry.registry
                ),
                UpdateAction::Conflicted { missing_base } => {
                    conflicts += 1;
                    println!(
                        "{} {name} {file} {would}merged with conflicts from {}",
                        style("!").red(),
                        entry.registry
                    );
                    if let Some(reason) = missing_base {
                        println!(
                            "  {}",
                            style(format!(
                                "every difference is a conflict, because the installed source \
                                 could not be read: {reason}"
                            ))
                            .dim()
                        );
                    }
                }
                UpdateAction::Skipped { reason } => println!(
                    "{}",
                    style(format!("- {name} {file} skipped: {reason}")).dim()
                ),
            }
            changed = true;
        }

        if !changed {
            println!("{} already up to date", style("✓").green());
        }
        if versions > 0 {
            println!(
                "{}",
                style(format!(
                    "{} the registry version of {versions} {} in components.toml",
                    if self.dry_run {
                        "would record"
                    } else {
                        "recorded"
                    },
                    if versions == 1 { "file" } else { "files" }
                ))
                .dim()
            );
        }
        if conflicts > 0 && !self.dry_run {
            println!();
            println!(
                "{} resolve the conflict markers in {conflicts} {}",
                style("!").red(),
                if conflicts == 1 { "file" } else { "files" }
            );
        }
        Ok(conflicts > 0 && !self.dry_run)
    }
}
