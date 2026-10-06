use std::path::PathBuf;

use clap::Args;
use console::style;
use topcoat_ui::manage::{self, InitOptions, Package};

use super::PackageArg;
use crate::common::prompt;

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
            if error != prompt::CANCELLED {
                eprintln!("{}", style(error).red());
            }
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

        // The theme's `--font-sans` expects the Geist family to be available.
        // Point the user at topcoat-font to provide it.
        println!();
        println!(
            "{} This theme uses the {} font. Set it up with topcoat-font:",
            style("!").yellow(),
            style("Geist").bold(),
        );
        println!(
            "  {} enable topcoat's {} feature",
            style("-").dim(),
            style("font-fontsource").bold(),
        );
        println!("  {} load it in your page's <head>:", style("-").dim());
        println!(
            "{}",
            style("      topcoat::font::link(font: fontsource_font!(GEIST))").dim(),
        );
        Ok(())
    }
}

/// Asks the user to select a theme with the arrow keys and Enter. Returns an error if
/// input is cancelled or no terminal is available. Non-interactive callers must select
/// a theme with `--theme`.
fn choose_theme(themes: &[String]) -> Result<String, String> {
    if !prompt::is_interactive() {
        return Err(format!(
            "no theme selected and no terminal to prompt on; pass --theme <name> (available: {})",
            themes.join(", ")
        ));
    }

    let mut select = cliclack::select("Choose a theme");
    for theme in themes {
        select = select.item(theme.clone(), theme, "");
    }
    select.interact().map_err(|error| prompt::error(&error))
}
