use std::path::{Path, PathBuf};

use clap::ValueEnum;
use console::style;
use dialoguer::{Confirm, Input as TextInput, Select, theme::ColorfulTheme};

use super::{
    check_destination,
    choice::{Font, Icons},
    input::{
        Answer, DEFAULT_ICON_SET, Input, Origin, Preset, Question, Sourced, font_family_id,
        icon_set_id,
    },
};

/// Asks for the choices that flags and presets leave open, in the terminal.
pub struct Wizard {
    theme: ColorfulTheme,
}

impl Wizard {
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
        }
    }

    /// Asks for the directory to create the application in. Unless `named` is set, the
    /// directory name must also be a valid package name.
    pub fn destination(&self, named: bool) -> Result<PathBuf, String> {
        let path: String = TextInput::with_theme(&self.theme)
            .with_prompt("Where should the application be created?")
            .default("my-app".to_string())
            .validate_with(|path: &String| check_destination(Path::new(path), named))
            .interact_text()
            .map_err(|error| read_error(&error))?;
        Ok(PathBuf::from(path))
    }

    /// Asks every open question of `input`, then the settings of integrations chosen
    /// here. Settings of integrations chosen with flags keep their defaults.
    pub fn ask(&self, input: &mut Input) -> Result<(), String> {
        let recommended = Preset::Recommended.choices();
        while let Some(&question) = input.questions().first() {
            let ui_open = input.ui.is_none();
            let answer = match question {
                Question::Routing => Answer::Routing(self.select(
                    "How should paths be assigned to handlers?",
                    recommended.routing,
                )?),
                Question::Database => {
                    Answer::Database(self.select("Database integration", recommended.database)?)
                }
                Question::Interaction => Answer::Interaction(
                    self.select("Browser interaction", recommended.interaction)?,
                ),
                Question::Tailwind => Answer::Tailwind(
                    self.confirm("Style pages with Tailwind CSS?", recommended.tailwind)?,
                ),
                Question::Icons => Answer::Icons(self.select("Icons", recommended.icons)?),
                Question::Font => Answer::Font(self.select("Font", recommended.font)?),
                Question::Ui => {
                    Answer::Ui(self.confirm("Install Topcoat UI components?", recommended.ui)?)
                }
            };
            input.answer(answer);

            if ui_open && input.ui.is_some_and(|ui| ui.origin == Origin::Inferred) {
                let missing = if input.tailwind.is_some_and(|tailwind| !tailwind.value) {
                    "Tailwind"
                } else {
                    "Iconify icons"
                };
                eprintln!(
                    "{}",
                    style(format!(
                        "Topcoat UI is unavailable because it requires {missing}"
                    ))
                    .dim()
                );
            }
        }

        if let Some(Sourced {
            value: Icons::Iconify,
            origin: Origin::Prompt,
        }) = input.icons
            && input.icon_set.is_none()
        {
            input.icon_set = Some(self.text("Iconify icon set", DEFAULT_ICON_SET, icon_set_id)?);
        }
        if let Some(Sourced {
            value: Font::Fontsource,
            origin: Origin::Prompt,
        }) = input.font
            && input.font_family.is_none()
        {
            let default = input.default_font_family();
            input.font_family =
                Some(self.text("Fontsource font family", default, font_family_id)?);
        }
        Ok(())
    }

    /// Asks for one of the values of `T`, described by their help text, with
    /// `recommended` selected initially.
    fn select<T: ValueEnum + Copy + PartialEq>(
        &self,
        prompt: &str,
        recommended: T,
    ) -> Result<T, String> {
        let variants = T::value_variants();
        let items: Vec<String> = variants
            .iter()
            .map(|variant| {
                let value = variant
                    .to_possible_value()
                    .expect("choice enums have no skipped variants");
                let label = value
                    .get_help()
                    .map_or_else(|| value.get_name().to_string(), ToString::to_string);
                if *variant == recommended {
                    format!("{label} {}", style("(recommended)").dim())
                } else {
                    label
                }
            })
            .collect();
        let default = variants
            .iter()
            .position(|variant| *variant == recommended)
            .unwrap_or_default();

        let index = Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(&items)
            .default(default)
            .interact_opt()
            .map_err(|error| read_error(&error))?
            .ok_or_else(cancelled)?;
        Ok(variants[index])
    }

    /// Asks a yes/no question with `recommended` as the default answer.
    fn confirm(&self, prompt: &str, recommended: bool) -> Result<bool, String> {
        Confirm::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(recommended)
            .interact_opt()
            .map_err(|error| read_error(&error))?
            .ok_or_else(cancelled)
    }

    /// Asks for text, with `default` as the default answer, until `parse` accepts it.
    fn text(
        &self,
        prompt: &str,
        default: &str,
        parse: fn(&str) -> Result<String, String>,
    ) -> Result<String, String> {
        let value: String = TextInput::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(default.to_string())
            .validate_with(|value: &String| parse(value).map(drop))
            .interact_text()
            .map_err(|error| read_error(&error))?;
        parse(&value)
    }
}

fn read_error(error: &dialoguer::Error) -> String {
    format!("failed to read input: {error}")
}

fn cancelled() -> String {
    "cancelled".to_string()
}
