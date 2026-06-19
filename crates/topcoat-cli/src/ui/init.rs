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
    /// Theme to install; when omitted, you are prompted to choose one
    #[arg(short, long)]
    theme: Option<String>,
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
            theme: self.theme,
        };

        let mut choose = choose_theme;
        let initialized = manage::init(&project, options, &mut choose)?;

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
        println!(
            "{} installed theme {} {}",
            style("+").green(),
            style(initialized.theme.name).bold(),
            style(format!("({})", initialized.theme.file.display())).dim(),
        );
        Ok(())
    }
}

/// Prompts the user to pick a theme from those the registry offers. A theme is
/// mandatory, so this loops until a valid choice is made and errors when there
/// is no terminal to prompt on (non-interactive use must pass `--theme`).
fn choose_theme(themes: &[String]) -> Result<String, String> {
    use std::io::{IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "no theme selected and no terminal to prompt on; pass --theme <name> (available: {})",
            themes.join(", ")
        ));
    }

    eprintln!("{}", style("Choose a theme:").bold());
    for (index, theme) in themes.iter().enumerate() {
        eprintln!("  {} {}", style(format!("{})", index + 1)).dim(), theme);
    }

    loop {
        eprint!(
            "{} ",
            style(format!("Theme [1-{}]:", themes.len())).yellow()
        );
        std::io::stderr().flush().ok();

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|error| format!("failed to read input: {error}"))?;
        let input = input.trim();

        // Accept either the number or the theme name itself.
        if let Ok(choice) = input.parse::<usize>()
            && (1..=themes.len()).contains(&choice)
        {
            return Ok(themes[choice - 1].clone());
        }
        if let Some(theme) = themes.iter().find(|name| name.as_str() == input) {
            return Ok(theme.clone());
        }

        eprintln!(
            "{}",
            style(format!("'{input}' is not one of the choices")).red()
        );
    }
}
