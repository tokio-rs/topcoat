use std::path::PathBuf;

use clap::Args;
use console::style;
use topcoat_ui::manage::{self, InitOptions, Package};

use super::PackageArg;

#[derive(Args)]
pub(super) struct InitCommand {
    /// Base directory for component install output (defaults to `src/components`)
    #[arg(short, long)]
    components_dir: Option<PathBuf>,
    /// Theme to install; when omitted, the sole theme is used, or you are
    /// prompted to choose when several are available
    #[arg(short, long)]
    theme: Option<String>,
    #[command(flatten)]
    package: PackageArg,
}

impl InitCommand {
    pub(super) fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let package = Package::locate(self.package.package)?;
        let options = InitOptions {
            components_dir: self.components_dir,
            theme: self.theme,
        };

        let mut choose = choose_theme;
        let initialized = manage::init(&package, options, &mut choose)?;

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
        let theme_file = initialized.theme.file.display().to_string();
        println!(
            "{} installed theme {} {}",
            style("+").green(),
            style(initialized.theme.name).bold(),
            style(format!("({theme_file})")).dim(),
        );

        // The installed stylesheet is the theme's Tailwind input (it carries the
        // `@import "tailwindcss"` and the theme tokens), so nothing loads until
        // the Tailwind build script is pointed at it. Tell the user to wire it up.
        println!();
        println!(
            "{} Load the theme by using it as your Tailwind {} in build.rs:",
            style("!").yellow(),
            style("input").bold(),
        );
        println!(
            "{}",
            style(format!(
                "      topcoat::tailwind::BuildConfig::new().input({theme_file:?}).render().unwrap();"
            ))
            .dim(),
        );

        // The theme's `--font-sans` expects the Lexend family to be available.
        // Point the user at topcoat-font to provide it.
        println!();
        println!(
            "{} This theme uses the {} font. Set it up with topcoat-font:",
            style("!").yellow(),
            style("Lexend").bold(),
        );
        println!(
            "  {} enable topcoat's {} feature",
            style("-").dim(),
            style("font-fontsource").bold(),
        );
        println!("  {} load it in your page's <head>:", style("-").dim());
        println!(
            "{}",
            style("      topcoat::font::link(font: fontsource_font!(LEXEND))").dim(),
        );
        Ok(())
    }
}

/// Prompts the user to pick a theme from those the registry offers, navigating
/// with the arrow keys and selecting with enter. A theme is mandatory, so this
/// errors when there is no terminal to prompt on (non-interactive use must pass
/// `--theme`) and when the prompt is cancelled (e.g. ctrl-c / esc).
fn choose_theme(themes: &[String]) -> Result<String, String> {
    use std::io::IsTerminal;

    use dialoguer::{Select, theme::ColorfulTheme};

    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "no theme selected and no terminal to prompt on; pass --theme <name> (available: {})",
            themes.join(", ")
        ));
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a theme")
        .items(themes)
        .default(0)
        .interact_opt()
        .map_err(|error| format!("failed to read input: {error}"))?;

    match selection {
        Some(index) => Ok(themes[index].clone()),
        None => Err("no theme selected".to_string()),
    }
}
