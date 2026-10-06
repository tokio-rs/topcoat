use clap::Args;
use topcoat_font::fontsource::Family;

use super::{
    choice::{Database, DatabaseBackend, Font, Icons, Interaction, Routing},
    options::{DatabaseSetup, FontSetup, IconSetup, ProjectOptions, UI_ICON_SET},
};

/// The Iconify set used when none is named.
pub const DEFAULT_ICON_SET: &str = "lucide";
/// The Fontsource family used when none is named.
const DEFAULT_FONT_FAMILY: &str = "inter";
/// The Fontsource family used with Topcoat UI when none is named, matching the UI theme.
const UI_FONT_FAMILY: &str = "geist";

/// Command-line flags for the application choices.
#[derive(Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is a command-line switch"
)]
pub struct ChoiceArgs {
    /// Use the recommended choice for every option not given as a flag
    #[arg(long, conflicts_with = "minimal")]
    recommended: bool,
    /// Use the smallest setup for every option not given as a flag
    #[arg(long)]
    minimal: bool,
    /// How request paths are assigned to handlers
    #[arg(long, value_name = "STYLE")]
    routing: Option<Routing>,
    /// Database integration
    #[arg(long, value_name = "INTEGRATION")]
    database: Option<Database>,
    /// Database backend [default: sqlite]
    #[arg(long, value_name = "BACKEND")]
    database_backend: Option<DatabaseBackend>,
    /// Browser interaction approach
    #[arg(long, value_name = "APPROACH")]
    interaction: Option<Interaction>,
    /// Style pages with Tailwind CSS
    #[arg(long, conflicts_with = "no_tailwind")]
    tailwind: bool,
    /// Style pages with a plain CSS stylesheet
    #[arg(long)]
    no_tailwind: bool,
    /// Icon source
    #[arg(long, value_name = "SOURCE")]
    icons: Option<Icons>,
    /// Iconify set to stage [default: lucide]
    #[arg(long, value_name = "SET")]
    icon_set: Option<String>,
    /// Font source
    #[arg(long, value_name = "SOURCE")]
    font: Option<Font>,
    /// Fontsource family ID [default: inter, or geist with UI]
    #[arg(long, value_name = "FAMILY")]
    font_family: Option<String>,
    /// Install Topcoat UI components (requires Tailwind and Iconify)
    #[arg(long, conflicts_with = "no_ui")]
    ui: bool,
    /// Do not install Topcoat UI components
    #[arg(long)]
    no_ui: bool,
}

impl ChoiceArgs {
    /// The preset selected by `--recommended` or `--minimal`, if any.
    pub fn preset(&self) -> Option<Preset> {
        if self.recommended {
            Some(Preset::Recommended)
        } else if self.minimal {
            Some(Preset::Minimal)
        } else {
            None
        }
    }
}

/// A complete set of top-level choices selected by a single flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Recommended,
    Minimal,
}

impl Preset {
    /// The preset's value for every top-level choice.
    pub fn choices(self) -> PresetChoices {
        match self {
            Self::Recommended => PresetChoices {
                routing: Routing::Module,
                database: Database::Toasty,
                interaction: Interaction::Topcoat,
                tailwind: true,
                icons: Icons::Iconify,
                font: Font::Fontsource,
                ui: true,
            },
            Self::Minimal => PresetChoices {
                routing: Routing::Module,
                database: Database::None,
                interaction: Interaction::None,
                tailwind: false,
                icons: Icons::None,
                font: Font::None,
                ui: false,
            },
        }
    }
}

/// A value for every top-level choice.
pub struct PresetChoices {
    pub routing: Routing,
    pub database: Database,
    pub interaction: Interaction,
    pub tailwind: bool,
    pub icons: Icons,
    pub font: Font,
    pub ui: bool,
}

/// Where a choice's value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// An explicit command-line flag.
    Flag,
    /// A preset's value for a choice not given as a flag.
    Preset(Preset),
    /// A prerequisite of another choice.
    Inferred,
    /// An answer to a wizard question.
    Prompt,
}

/// A choice's value and where it came from.
#[derive(Clone, Copy, Debug)]
pub struct Sourced<T> {
    pub value: T,
    pub origin: Origin,
}

impl<T> Sourced<T> {
    pub fn new(value: T, origin: Origin) -> Self {
        Self { value, origin }
    }
}

/// Application choices collected so far. A `None` field is still open.
pub struct Input {
    pub routing: Option<Sourced<Routing>>,
    pub database: Option<Sourced<Database>>,
    pub database_backend: Option<DatabaseBackend>,
    pub interaction: Option<Sourced<Interaction>>,
    pub tailwind: Option<Sourced<bool>>,
    pub icons: Option<Sourced<Icons>>,
    pub icon_set: Option<String>,
    pub font: Option<Sourced<Font>>,
    pub font_family: Option<String>,
    pub ui: Option<Sourced<bool>>,
}

impl Input {
    /// Collects the choices made by `args`: explicit flags first, then the selected
    /// preset for choices the flags leave open, then prerequisites of the result.
    pub fn new(args: ChoiceArgs) -> Self {
        fn flag<T>(value: T) -> Sourced<T> {
            Sourced::new(value, Origin::Flag)
        }

        let preset = args.preset();
        let mut input = Self {
            routing: args.routing.map(flag),
            database: args.database.map(flag),
            database_backend: args.database_backend,
            interaction: args.interaction.map(flag),
            tailwind: switch(args.tailwind, args.no_tailwind).map(flag),
            icons: args.icons.map(flag),
            icon_set: args.icon_set,
            font: args.font.map(flag),
            font_family: args.font_family,
            ui: switch(args.ui, args.no_ui).map(flag),
        };
        if let Some(preset) = preset {
            input.fill(preset);
        }
        input.infer();
        input
    }

    /// Fills open top-level choices from `preset`.
    fn fill(&mut self, preset: Preset) {
        let origin = Origin::Preset(preset);
        let choices = preset.choices();
        self.routing
            .get_or_insert(Sourced::new(choices.routing, origin));
        self.database
            .get_or_insert(Sourced::new(choices.database, origin));
        self.interaction
            .get_or_insert(Sourced::new(choices.interaction, origin));
        self.tailwind
            .get_or_insert(Sourced::new(choices.tailwind, origin));
        self.icons
            .get_or_insert(Sourced::new(choices.icons, origin));
        self.font.get_or_insert(Sourced::new(choices.font, origin));
        self.ui.get_or_insert(Sourced::new(choices.ui, origin));
    }

    /// Fills open choices that follow from the choices already made. Topcoat UI
    /// requires Tailwind and Iconify, and is unavailable without either.
    pub fn infer(&mut self) {
        if self.ui.is_some_and(|ui| ui.value) {
            self.tailwind
                .get_or_insert(Sourced::new(true, Origin::Inferred));
            self.icons
                .get_or_insert(Sourced::new(Icons::Iconify, Origin::Inferred));
        }

        let ui_unavailable = self.tailwind.is_some_and(|tailwind| !tailwind.value)
            || self
                .icons
                .is_some_and(|icons| icons.value != Icons::Iconify);
        if ui_unavailable {
            self.ui.get_or_insert(Sourced::new(false, Origin::Inferred));
        }
    }

    /// Records the answer to a wizard question, then fills the choices that follow from
    /// it.
    pub fn answer(&mut self, answer: Answer) {
        fn prompt<T>(value: T) -> Sourced<T> {
            Sourced::new(value, Origin::Prompt)
        }

        match answer {
            Answer::Routing(value) => self.routing = Some(prompt(value)),
            Answer::Database(value) => self.database = Some(prompt(value)),
            Answer::Interaction(value) => self.interaction = Some(prompt(value)),
            Answer::Tailwind(value) => self.tailwind = Some(prompt(value)),
            Answer::Icons(value) => self.icons = Some(prompt(value)),
            Answer::Font(value) => self.font = Some(prompt(value)),
            Answer::Ui(value) => self.ui = Some(prompt(value)),
        }
        self.infer();
    }

    /// The Fontsource family used when none is named: the UI theme's font if Topcoat UI
    /// is selected.
    pub fn default_font_family(&self) -> &'static str {
        if self.ui.is_some_and(|ui| ui.value) {
            UI_FONT_FAMILY
        } else {
            DEFAULT_FONT_FAMILY
        }
    }

    /// The open top-level choices, in the order the wizard asks them.
    pub fn questions(&self) -> Vec<Question> {
        [
            (self.routing.is_none(), Question::Routing),
            (self.database.is_none(), Question::Database),
            (self.interaction.is_none(), Question::Interaction),
            (self.tailwind.is_none(), Question::Tailwind),
            (self.icons.is_none(), Question::Icons),
            (self.font.is_none(), Question::Font),
            (self.ui.is_none(), Question::Ui),
        ]
        .into_iter()
        .filter_map(|(open, question)| open.then_some(question))
        .collect()
    }

    /// Validates the complete set of choices and fills defaults for unset settings.
    ///
    /// Returns an error naming the missing flags if a top-level choice is still open,
    /// or explaining how to resolve choices that cannot be combined.
    pub fn resolve(&self) -> Result<Resolution, String> {
        let (
            Some(routing),
            Some(database),
            Some(interaction),
            Some(tailwind),
            Some(icons),
            Some(font),
            Some(ui),
        ) = (
            self.routing,
            self.database,
            self.interaction,
            self.tailwind,
            self.icons,
            self.font,
            self.ui,
        )
        else {
            let flags: Vec<&str> = self.questions().into_iter().map(Question::flag).collect();
            return Err(format!(
                "missing choices: pass {}, or use --recommended or --minimal",
                flags.join(", ")
            ));
        };

        if ui.value && !tailwind.value {
            return Err(conflict(
                "Topcoat UI requires Tailwind",
                [(ui.origin, "--no-ui"), (tailwind.origin, "--tailwind")],
            ));
        }
        if ui.value && icons.value != Icons::Iconify {
            return Err(conflict(
                "Topcoat UI requires Iconify icons",
                [(ui.origin, "--no-ui"), (icons.origin, "--icons iconify")],
            ));
        }

        let database_setup = match (database.value, self.database_backend) {
            (Database::Toasty, backend) => DatabaseSetup::Toasty {
                backend: backend.unwrap_or_default(),
            },
            (Database::None, Some(_)) => {
                return Err("--database-backend requires --database toasty".to_string());
            }
            (Database::None, None) => DatabaseSetup::None,
        };

        let icon_setup = match (icons.value, self.icon_set.as_deref()) {
            (Icons::Iconify, set) => IconSetup::Iconify {
                set: icon_set_id(set.unwrap_or(DEFAULT_ICON_SET))?,
            },
            (Icons::Custom | Icons::None, Some(_)) => {
                return Err("--icon-set requires --icons iconify".to_string());
            }
            (Icons::Custom, None) => IconSetup::Custom,
            (Icons::None, None) => IconSetup::None,
        };

        let font_setup = match (font.value, self.font_family.as_deref()) {
            (Font::Fontsource, family) => {
                let family = family.unwrap_or(self.default_font_family());
                FontSetup::Fontsource {
                    family: font_family_id(family)?,
                }
            }
            (Font::None, Some(_)) => {
                return Err("--font-family requires --font fontsource".to_string());
            }
            (Font::None, None) => FontSetup::None,
        };

        let options = ProjectOptions {
            routing: routing.value,
            database: database_setup,
            interaction: interaction.value,
            tailwind: tailwind.value,
            icons: icon_setup,
            font: font_setup,
            ui: ui.value,
        };

        let mut notes = Vec::new();
        if tailwind.origin == Origin::Inferred {
            notes.push("Tailwind is enabled because Topcoat UI requires it".to_string());
        }
        if icons.origin == Origin::Inferred {
            notes.push("Iconify icons are enabled because Topcoat UI requires them".to_string());
        }
        if options.ui && options.font == FontSetup::None {
            notes.push(
                "the Topcoat UI theme expects the Geist font and falls back to the system font"
                    .to_string(),
            );
        }
        if options.ui && !matches!(&options.icons, IconSetup::Iconify { set } if set == UI_ICON_SET)
        {
            notes.push(format!(
                "the `{UI_ICON_SET}` icon set is also staged because Topcoat UI components use it"
            ));
        }

        Ok(Resolution { options, notes })
    }
}

/// An open top-level choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    Routing,
    Database,
    Interaction,
    Tailwind,
    Icons,
    Font,
    Ui,
}

impl Question {
    /// The flags that answer the question.
    pub fn flag(self) -> &'static str {
        match self {
            Self::Routing => "--routing",
            Self::Database => "--database",
            Self::Interaction => "--interaction",
            Self::Tailwind => "--tailwind or --no-tailwind",
            Self::Icons => "--icons",
            Self::Font => "--font",
            Self::Ui => "--ui or --no-ui",
        }
    }
}

/// An answer to a wizard question.
#[derive(Clone, Copy, Debug)]
pub enum Answer {
    Routing(Routing),
    Database(Database),
    Interaction(Interaction),
    Tailwind(bool),
    Icons(Icons),
    Font(Font),
    Ui(bool),
}

/// Validated options and notes about choices made on the user's behalf.
pub struct Resolution {
    pub options: ProjectOptions,
    pub notes: Vec<String>,
}

/// Combines a pair of `--x` and `--no-x` flags. Clap rejects passing both.
fn switch(enabled: bool, disabled: bool) -> Option<bool> {
    match (enabled, disabled) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

/// Describes two choices that cannot be combined. `overrides` pairs each choice's
/// origin with the flag that changes it; only choices not given as explicit flags are
/// suggested for overriding.
fn conflict(message: &str, overrides: [(Origin, &str); 2]) -> String {
    let suggestions: Vec<&str> = overrides
        .iter()
        .filter(|(origin, _)| *origin != Origin::Flag)
        .map(|(_, flag)| *flag)
        .collect();
    if suggestions.is_empty() {
        format!("{message}, so these flags cannot be combined")
    } else {
        format!("{message}; also pass {}", suggestions.join(" or "))
    }
}

/// Checks that `value` has the form of an Iconify set ID such as `lucide`: lowercase
/// ASCII letters, digits, and hyphens. Whether the set exists is only known once it is
/// downloaded.
pub fn icon_set_id(value: &str) -> Result<String, String> {
    let valid = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(value.to_string())
    } else {
        Err(format!(
            "invalid icon set `{value}`: use lowercase letters, digits, and hyphens"
        ))
    }
}

/// Checks that `value` is the ID of a family in the Fontsource catalog, such as `inter`.
pub fn font_family_id(value: &str) -> Result<String, String> {
    if Family::by_id(value).is_some() {
        Ok(value.to_string())
    } else {
        Err(format!(
            "unknown Fontsource family `{value}`; use a family ID listed at \
             https://fontsource.org, such as `{DEFAULT_FONT_FAMILY}`"
        ))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        choices: ChoiceArgs,
    }

    fn input<S: AsRef<str>>(args: &[S]) -> Input {
        let args = std::iter::once("topcoat-new").chain(args.iter().map(AsRef::as_ref));
        Input::new(Cli::try_parse_from(args).unwrap().choices)
    }

    fn resolve<S: AsRef<str>>(args: &[S]) -> Result<ProjectOptions, String> {
        input(args).resolve().map(|resolution| resolution.options)
    }

    #[test]
    fn presets_answer_every_question() {
        for preset in ["--recommended", "--minimal"] {
            assert_eq!(input(&[preset]).questions(), []);
            assert!(resolve(&[preset]).is_ok());
        }
    }

    #[test]
    fn flags_override_preset_choices() {
        let options = resolve(&[
            "--recommended",
            "--database",
            "none",
            "--interaction",
            "htmx",
        ])
        .unwrap();
        assert_eq!(options.database, DatabaseSetup::None);
        assert_eq!(options.interaction, Interaction::Htmx);
    }

    #[test]
    fn preset_conflicts_suggest_overriding_the_preset_choice() {
        let error = resolve(&["--recommended", "--no-tailwind"]).unwrap_err();
        assert!(error.contains("--no-ui"));
        assert!(resolve(&["--recommended", "--no-tailwind", "--no-ui"]).is_ok());

        let error = resolve(&["--minimal", "--ui"]).unwrap_err();
        assert!(error.contains("--tailwind"));
        let error = resolve(&["--minimal", "--ui", "--tailwind"]).unwrap_err();
        assert!(error.contains("--icons iconify"));
        assert!(resolve(&["--minimal", "--ui", "--tailwind", "--icons", "iconify"]).is_ok());
    }

    #[test]
    fn explicitly_conflicting_flags_are_rejected() {
        assert!(resolve(&["--recommended", "--ui", "--no-tailwind"]).is_err());
        assert!(resolve(&["--recommended", "--ui", "--icons", "custom"]).is_err());
    }

    #[test]
    fn ui_answers_its_prerequisites() {
        let questions = input(&["--ui"]).questions();
        assert!(!questions.contains(&Question::Tailwind));
        assert!(!questions.contains(&Question::Icons));
        assert!(questions.contains(&Question::Database));
    }

    #[test]
    fn ui_is_not_asked_once_unavailable() {
        assert!(
            !input(&["--no-tailwind"])
                .questions()
                .contains(&Question::Ui)
        );
        assert!(
            !input(&["--icons", "none"])
                .questions()
                .contains(&Question::Ui)
        );
        assert!(input(&["--tailwind"]).questions().contains(&Question::Ui));
    }

    #[test]
    fn answers_resolve_like_the_equivalent_flags() {
        let mut answered = input::<&str>(&[]);
        for answer in [
            Answer::Routing(Routing::Discover),
            Answer::Database(Database::Toasty),
            Answer::Interaction(Interaction::Htmx),
            Answer::Tailwind(true),
            Answer::Icons(Icons::Custom),
            Answer::Font(Font::Fontsource),
        ] {
            answered.answer(answer);
        }
        // Custom icons make Topcoat UI unavailable, so it is not asked.
        assert_eq!(answered.questions(), []);

        let flags = resolve(&[
            "--routing",
            "discover",
            "--database",
            "toasty",
            "--interaction",
            "htmx",
            "--tailwind",
            "--icons",
            "custom",
            "--font",
            "fontsource",
            "--no-ui",
        ])
        .unwrap();
        assert_eq!(answered.resolve().unwrap().options, flags);
    }

    #[test]
    fn answering_no_tailwind_makes_ui_unavailable() {
        let mut input = input(&["--icons", "iconify"]);
        assert!(input.questions().contains(&Question::Ui));
        input.answer(Answer::Tailwind(false));
        assert!(!input.questions().contains(&Question::Ui));
    }

    #[test]
    fn missing_choices_name_their_flags() {
        let error = resolve(&["--database", "none", "--no-ui"]).unwrap_err();
        assert!(error.contains("--interaction"));
        assert!(!error.contains("--database"));
    }

    #[test]
    fn settings_require_their_integration() {
        for setting in [
            ["--database-backend", "sqlite"],
            ["--icon-set", "tabler"],
            ["--font-family", "inter"],
        ] {
            let mut args = vec!["--minimal"];
            args.extend(setting);
            assert!(resolve(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn settings_reject_ids_that_are_not_catalog_ids() {
        assert!(resolve(&["--recommended", "--icon-set", "../lucide"]).is_err());
        assert!(resolve(&["--recommended", "--font-family", "\"inter\""]).is_err());
    }

    #[test]
    fn font_families_must_exist_in_the_fontsource_catalog() {
        assert!(resolve(&["--recommended", "--font-family", "roboto"]).is_ok());
        assert!(resolve(&["--recommended", "--font-family", "no-such-font"]).is_err());
    }

    #[test]
    fn ui_stages_its_icon_set_alongside_the_chosen_one() {
        let options = resolve(&["--recommended", "--icon-set", "tabler"]).unwrap();
        let mut sets = options.icon_sets();
        sets.sort_unstable();
        assert_eq!(sets, [UI_ICON_SET, "tabler"]);
    }

    #[test]
    fn equivalent_arguments_reproduce_the_options() {
        let cases: [&[&str]; 3] = [
            &["--recommended"],
            &["--minimal"],
            &[
                "--minimal",
                "--routing",
                "manual",
                "--icons",
                "iconify",
                "--icon-set",
                "tabler",
                "--font",
                "fontsource",
            ],
        ];
        for args in cases {
            let options = resolve(args).unwrap();
            let equivalent = options.to_args();
            assert_eq!(input(&equivalent).questions(), [], "{equivalent:?}");
            assert_eq!(resolve(&equivalent).unwrap(), options, "{equivalent:?}");
        }
    }
}
