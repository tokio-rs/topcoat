use std::path::{Path, PathBuf};

use topcoat_font::fontsource::families;

use super::{
    check_destination,
    choice::{Database, Font, Icons, Interaction, Routing},
    input::{
        Answer, DEFAULT_ICON_SET, Input, Origin, Preset, Question, Sourced, font_family_id,
        icon_set_id,
    },
};
use crate::common::prompt;

/// Widely used Iconify sets, suggested while typing an icon set.
const POPULAR_ICON_SETS: &[&str] = &[
    "lucide",
    "tabler",
    "ph",
    "heroicons",
    "mdi",
    "material-symbols",
    "carbon",
    "ri",
    "bi",
    "simple-icons",
];

/// The hint of every `None` option: the integration is left to the application.
const BRING_YOUR_OWN: &str = "set it up yourself";

/// Asks for the directory to create the application in. Unless `named` is set, the
/// directory name must also be a valid package name.
pub fn destination(named: bool) -> Result<PathBuf, String> {
    let path: String = cliclack::input("Where should the application be created?")
        .default_input("my-app")
        .validate(move |path: &String| check_destination(Path::new(path), named))
        .interact()
        .map_err(|error| prompt::error(&error))?;
    Ok(PathBuf::from(path))
}

/// Asks every open question of `input`, then the settings of integrations chosen
/// here. Settings of integrations chosen with flags keep their defaults.
pub fn ask(input: &mut Input) -> Result<(), String> {
    let recommended = Preset::Recommended.choices();
    while let Some(&question) = input.questions().first() {
        let answer = match question {
            Question::Routing => Answer::Routing(select(
                "How should paths be assigned to handlers?",
                &[
                    (Routing::Module, "Module", "paths follow the module tree"),
                    (
                        Routing::Discover,
                        "Discover",
                        "explicit paths, collected automatically",
                    ),
                    (
                        Routing::Manual,
                        "Manual",
                        "explicit paths, registered by hand",
                    ),
                ],
                recommended.routing,
            )?),
            Question::Database => Answer::Database(select(
                "Which database integration?",
                &[
                    (Database::Toasty, "Toasty", "async ORM"),
                    (Database::None, "None", BRING_YOUR_OWN),
                ],
                recommended.database,
            )?),
            Question::Interaction => Answer::Interaction(select(
                "Which browser interaction library?",
                &[
                    (
                        Interaction::Topcoat,
                        "Topcoat runtime",
                        "signals, shards, and procedures",
                    ),
                    (
                        Interaction::Htmx,
                        "htmx",
                        "hx-* attributes and HTML fragments",
                    ),
                    (
                        Interaction::Datastar,
                        "Datastar",
                        "signals and streamed patches",
                    ),
                    (
                        Interaction::AlpineAjax,
                        "Alpine AJAX",
                        "Alpine with AJAX form targets",
                    ),
                    (Interaction::None, "None", BRING_YOUR_OWN),
                ],
                recommended.interaction,
            )?),
            Question::Ui => Answer::Ui(confirm(
                "Use Topcoat UI components? They come with Tailwind CSS, Lucide icons, and the Geist font",
                recommended.ui,
            )?),
            Question::Tailwind => {
                Answer::Tailwind(confirm("Use Tailwind CSS?", recommended.tailwind)?)
            }
            Question::Icons => Answer::Icons(select(
                "Which icon integration?",
                &[
                    (
                        Icons::Iconify,
                        "Iconify",
                        "icon sets checked at compile time",
                    ),
                    (Icons::Custom, "Custom", "hand-written SVG icons"),
                    (Icons::None, "None", BRING_YOUR_OWN),
                ],
                recommended.icons,
            )?),
            Question::Font => Answer::Font(select(
                "Which font integration?",
                &[
                    (
                        Font::Fontsource,
                        "Fontsource",
                        "web fonts from the Fontsource catalog",
                    ),
                    (Font::None, "None", BRING_YOUR_OWN),
                ],
                recommended.font,
            )?),
        };
        input.answer(answer);
    }

    if let Some(Sourced {
        value: Icons::Iconify,
        origin: Origin::Prompt,
    }) = input.icons
        && input.icon_set.is_none()
    {
        let suggestions = POPULAR_ICON_SETS.iter().map(ToString::to_string).collect();
        input.icon_set = Some(text(
            "Which Iconify set?",
            DEFAULT_ICON_SET,
            suggestions,
            icon_set_id,
        )?);
    }
    if let Some(Sourced {
        value: Font::Fontsource,
        origin: Origin::Prompt,
    }) = input.font
        && input.font_family.is_none()
    {
        let suggestions = families::ALL
            .iter()
            .map(|family| family.id.to_string())
            .collect();
        input.font_family = Some(text(
            "Which Fontsource family?",
            input.default_font_family(),
            suggestions,
            font_family_id,
        )?);
    }
    Ok(())
}

/// Asks for one of `items`, each a value with its label and hint, with `recommended`
/// selected initially.
fn select<T: Copy + Eq + 'static>(
    prompt: &str,
    items: &[(T, &str, &str)],
    recommended: T,
) -> Result<T, String> {
    let mut select = cliclack::select(prompt).initial_value(recommended);
    for &(value, label, hint) in items {
        select = if value == recommended {
            select.item(value, label, format!("{hint}, recommended"))
        } else {
            select.item(value, label, hint)
        };
    }
    select.interact().map_err(|error| prompt::error(&error))
}

/// Asks a yes/no question with `recommended` as the initial answer.
fn confirm(prompt: &str, recommended: bool) -> Result<bool, String> {
    cliclack::confirm(prompt)
        .initial_value(recommended)
        .interact()
        .map_err(|error| prompt::error(&error))
}

/// Asks for text with `default` as the default answer and `suggestions` offered while
/// typing, until `parse` accepts it.
fn text(
    prompt: &str,
    default: &str,
    suggestions: Vec<String>,
    parse: fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let value: String = cliclack::input(prompt)
        .default_input(default)
        .autocomplete(suggestions)
        .validate(move |value: &String| parse(value).map(drop))
        .interact()
        .map_err(|error| prompt::error(&error))?;
    parse(&value)
}
