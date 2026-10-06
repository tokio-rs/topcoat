use std::{fmt::Write as _, path::Path};

use askama::Template;
use topcoat_core_grammar::pretty::pretty_print_str;
use topcoat_font::fontsource::{Family, Style};
use topcoat_ui::{
    Registry,
    manage::{self, ScaffoldOptions},
};

use super::{
    choice::{DatabaseBackend, Interaction, Routing},
    manifest::{Dependency, Manifest},
    name::PackageName,
    options::{DatabaseSetup, FontSetup, IconSetup, ProjectOptions},
    plan::ProjectPlan,
};
use crate::common::format;

/// The `topcoat` version generated applications depend on. The CLI is released together
/// with `topcoat`.
const TOPCOAT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The `tokio` version generated applications depend on.
const TOKIO_VERSION: &str = "1.51.1";
/// The `serde` version generated applications depend on.
const SERDE_VERSION: &str = "1";
/// The `toasty` and `toasty-cli` version generated applications depend on.
const TOASTY_VERSION: &str = "0.11";
/// The Iconify set the home page renders an example icon from.
const EXAMPLE_ICON_SET: &str = "lucide";
/// The stylesheet at the package root: plain CSS, the Tailwind input, or the UI theme.
const STYLESHEET: &str = "styles.css";
/// The Topcoat UI components the starter pages use.
const UI_COMPONENTS: &[&str] = &["button", "card", "input"];
/// The UI theme's declaration of its sans-serif font, naming the family it expects.
const THEME_FONT: &str = r#"--font-sans: "Geist", sans-serif;"#;

/// `src/main.rs`: the module declarations and entry point.
#[derive(Template)]
#[template(path = "base/src/main.rs.askama", escape = "none")]
struct MainRs {
    /// Whether the application stores its data with Toasty.
    toasty: bool,
    /// Whether the application has Topcoat UI components.
    ui: bool,
    /// Whether the application has a module of hand-written icons.
    custom_icons: bool,
}

/// `.gitignore`.
#[derive(Template)]
#[template(path = "base/.gitignore.askama", escape = "none")]
struct Gitignore {
    /// Whether the application stores its data with Toasty.
    toasty: bool,
}

/// `src/app.rs`: the router, root layout, and home page.
#[derive(Template)]
#[template(path = "base/src/app.rs.askama", escape = "none")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool switches a template section"
)]
struct AppRs<'a> {
    /// The page title as a Rust string literal.
    title: &'a str,
    /// Whether the home page uses Topcoat UI components.
    ui: bool,
    routing: Routing,
    /// Whether the router collects handlers, including fonts, with `.discover()`.
    discover: bool,
    paths: Paths,
    classes: Classes,
    interaction: Interaction,
    /// Whether the application stores its data with Toasty.
    toasty: bool,
    tailwind: bool,
    /// Whether the home page shows an icon from the default Iconify set.
    iconify_example: bool,
    /// The Iconify set to explain in a comment when it has no example icon.
    icon_set_hint: Option<&'a str>,
    /// Whether the application has a module of hand-written icons.
    custom_icons: bool,
    font: Option<FontInfo>,
}

/// The Fontsource family a generated application loads.
struct FontInfo {
    /// The name of the family's constant, e.g. `INTER`.
    ident: &'static str,
    /// The family's display name, e.g. `Inter`.
    name: &'static str,
    /// The arguments of `fontsource_font!` after the family, each preceded by `, `.
    args: String,
    /// The generic CSS family used until the font loads or if it fails to.
    generic: &'static str,
}

impl FontInfo {
    /// The weights the starter pages use: regular text and bold headings.
    const WEIGHTS: [u16; 2] = [400, 700];

    /// Describes `family`, limited to the starter's weights in the normal style where the
    /// family offers them, since every included face is preloaded.
    fn new(family: &Family) -> Self {
        let weights: Vec<String> = Self::WEIGHTS
            .iter()
            .filter(|weight| family.has_weight(**weight))
            .map(ToString::to_string)
            .collect();
        let mut args = String::new();
        if !weights.is_empty() {
            write!(args, ", weight: [{}]", weights.join(", "))
                .expect("writing to a String cannot fail");
        }
        if family.has_style(Style::Normal) {
            args.push_str(", style: Normal");
        }

        Self {
            ident: family.ident,
            name: family.name,
            args,
            generic: match family.category {
                "serif" => "serif",
                "monospace" => "monospace",
                _ => "sans-serif",
            },
        }
    }
}

/// Renders the arguments of handler attributes for the selected routing style.
#[derive(Clone, Copy)]
struct Paths(Routing);

impl Paths {
    /// The arguments of a `#[page]` or `#[layout]` attribute for a handler at `path`:
    /// empty with module routing, where paths come from modules.
    fn page(self, path: &str) -> String {
        match self.0 {
            Routing::Module => String::new(),
            Routing::Discover | Routing::Manual => format!("({path:?})"),
        }
    }

    /// The arguments of a `#[route]` attribute for a `method` handler at `path`.
    fn route(self, method: &str, path: &str) -> String {
        match self.0 {
            Routing::Module => format!("({method})"),
            Routing::Discover | Routing::Manual => format!("({method} {path:?})"),
        }
    }

    /// The arguments of a `#[route]` attribute for a `method` handler below its module:
    /// at `relative` to the module path with module routing, otherwise at `path`.
    fn route_at(self, method: &str, relative: &str, path: &str) -> String {
        match self.0 {
            Routing::Module => format!("({method} {relative:?})"),
            Routing::Discover | Routing::Manual => format!("({method} {path:?})"),
        }
    }

    /// The macro declaring a path parameter: `module_param` with module routing, where
    /// the parameter is also the module's segment, otherwise `path_param`.
    fn param(self) -> &'static str {
        if self.is_module() {
            "module_param"
        } else {
            "path_param"
        }
    }

    fn is_module(self) -> bool {
        self.0 == Routing::Module
    }
}

/// `class` attributes for the starter's elements, each with a leading space: Tailwind
/// utility classes, or nothing with the plain stylesheet.
#[derive(Clone, Copy)]
struct Classes {
    main: &'static str,
    heading: &'static str,
    paragraph: &'static str,
    link: &'static str,
    /// The form that creates a todo, holding its input and button.
    form: &'static str,
    /// Buttons, which use the button component instead with Topcoat UI.
    button: &'static str,
    /// Text inputs, which use the input component instead with Topcoat UI.
    input: &'static str,
    list: &'static str,
    item: &'static str,
}

impl Classes {
    /// Classes for the selected styling. With Topcoat UI, elements use the theme's colors.
    fn new(tailwind: bool, ui: bool) -> Self {
        if ui {
            Self {
                main: r#" class="mx-auto max-w-2xl px-4 py-16""#,
                heading: r#" class="text-3xl font-semibold tracking-tight""#,
                paragraph: r#" class="mt-4""#,
                link: r#" class="font-medium text-primary underline underline-offset-4""#,
                form: r#" class="mt-4 flex gap-2""#,
                button: "",
                input: "",
                list: r#" class="mt-4 space-y-2""#,
                item: r#" class="flex items-center gap-2""#,
            }
        } else if tailwind {
            Self {
                main: r#" class="mx-auto max-w-2xl px-4 py-16""#,
                heading: r#" class="text-3xl font-bold""#,
                paragraph: r#" class="mt-4""#,
                link: r#" class="text-blue-600 underline""#,
                form: r#" class="mt-4""#,
                button: r#" class="rounded border px-3 py-1""#,
                input: r#" class="rounded border px-2 py-1""#,
                list: r#" class="mt-4 space-y-2""#,
                item: r#" class="flex items-center gap-2""#,
            }
        } else {
            Self {
                main: "",
                heading: "",
                paragraph: "",
                link: "",
                form: "",
                button: "",
                input: "",
                list: "",
                item: "",
            }
        }
    }
}

/// `src/app/todos.rs` with plain HTML forms.
#[derive(Template)]
#[template(path = "interaction/forms/src/app/todos.rs.askama", escape = "none")]
struct FormsTodos {
    paths: Paths,
    classes: Classes,
    /// Whether buttons use the Topcoat UI button component.
    ui: bool,
}

/// `src/app/todos/id.rs` with plain HTML forms, also used with Alpine AJAX.
#[derive(Template)]
#[template(path = "interaction/forms/src/app/todos/id.rs.askama", escape = "none")]
struct FormsTodoId {
    paths: Paths,
}

/// `src/app/todos.rs` with Topcoat's browser runtime. Procedures replace the routes of
/// `src/app/todos/id.rs`.
#[derive(Template)]
#[template(path = "interaction/topcoat/src/app/todos.rs.askama", escape = "none")]
struct TopcoatTodos {
    paths: Paths,
    classes: Classes,
    /// Whether buttons use the Topcoat UI button component.
    ui: bool,
}

/// `src/app/todos.rs` with Alpine AJAX.
#[derive(Template)]
#[template(
    path = "interaction/alpine-ajax/src/app/todos.rs.askama",
    escape = "none"
)]
struct AlpineAjaxTodos {
    paths: Paths,
    classes: Classes,
    /// Whether buttons use the Topcoat UI button component.
    ui: bool,
}

/// `src/app/todos.rs` with htmx.
#[derive(Template)]
#[template(path = "interaction/htmx/src/app/todos.rs.askama", escape = "none")]
struct HtmxTodos {
    paths: Paths,
    classes: Classes,
    /// Whether buttons use the Topcoat UI button component.
    ui: bool,
}

/// `src/app/todos/id.rs` with htmx.
#[derive(Template)]
#[template(path = "interaction/htmx/src/app/todos/id.rs.askama", escape = "none")]
struct HtmxTodoId {
    paths: Paths,
}

/// `src/app/todos.rs` with Datastar.
#[derive(Template)]
#[template(path = "interaction/datastar/src/app/todos.rs.askama", escape = "none")]
struct DatastarTodos {
    paths: Paths,
    classes: Classes,
    /// Whether buttons use the Topcoat UI button component.
    ui: bool,
}

/// `src/app/todos/id.rs` with Datastar.
#[derive(Template)]
#[template(
    path = "interaction/datastar/src/app/todos/id.rs.askama",
    escape = "none"
)]
struct DatastarTodoId {
    paths: Paths,
}

/// `build.rs`: build steps of the selected integrations.
#[derive(Template)]
#[template(path = "base/build.rs.askama", escape = "none")]
struct BuildRs<'a> {
    tailwind: bool,
    icon_sets: Vec<&'a str>,
}

impl BuildRs<'_> {
    /// Whether any build step is selected. Without one, no build script is generated.
    fn is_needed(&self) -> bool {
        self.tailwind || !self.icon_sets.is_empty()
    }
}

#[derive(Template)]
#[template(path = "base/README.md.askama", escape = "none")]
struct Readme<'a> {
    name: &'a str,
    /// Whether the application stores its data with Toasty.
    toasty: bool,
}

/// Renders the files of a new application named `name` with the given options.
///
/// # Errors
///
/// Returns an error if the options select an integration that cannot be generated, or
/// if rendering or formatting a file fails.
pub fn generate(name: &PackageName, options: &ProjectOptions) -> Result<ProjectPlan, String> {
    let mut manifest = Manifest::new(name.clone());
    manifest.dependency(
        "tokio",
        Dependency::new(TOKIO_VERSION).features(["macros", "rt-multi-thread"]),
    )?;
    manifest.dependency(
        "topcoat",
        topcoat().features(["asset", "router", "serve", "view"]),
    )?;
    if options.routing != Routing::Manual {
        manifest.dependency("topcoat", topcoat().features(["discover"]))?;
    }
    if options.tailwind {
        manifest.dependency("topcoat", topcoat().features(["tailwind"]))?;
        manifest.build_dependency("topcoat", topcoat().features(["tailwind"]))?;
    }
    match &options.icons {
        IconSetup::None => {}
        IconSetup::Custom => manifest.dependency("topcoat", topcoat().features(["icon"]))?,
        IconSetup::Iconify { .. } => {
            manifest.dependency("topcoat", topcoat().features(["icon-iconify"]))?;
            manifest.build_dependency("topcoat", topcoat().features(["icon-iconify"]))?;
        }
    }

    match options.interaction {
        Interaction::None => {}
        Interaction::Topcoat => manifest.dependency("topcoat", topcoat().features(["runtime"]))?,
        Interaction::Htmx => manifest.dependency("topcoat", topcoat().features(["htmx"]))?,
        Interaction::Datastar => {
            manifest.dependency("topcoat", topcoat().features(["datastar"]))?;
        }
        Interaction::AlpineAjax => {
            manifest.dependency("topcoat", topcoat().features(["alpine-ajax"]))?;
        }
    }
    // Every todo page except the runtime's reads form or signal data with serde.
    if options.interaction != Interaction::Topcoat {
        manifest.dependency("serde", Dependency::new(SERDE_VERSION).features(["derive"]))?;
    }

    let toasty = match options.database {
        DatabaseSetup::None => false,
        DatabaseSetup::Toasty {
            backend: DatabaseBackend::Sqlite,
        } => {
            manifest.dependency(
                "toasty",
                Dependency::new(TOASTY_VERSION).features(["sqlite"]),
            )?;
            manifest.dependency("toasty-cli", Dependency::new(TOASTY_VERSION))?;
            true
        }
    };

    let font = match &options.font {
        FontSetup::None => None,
        FontSetup::Fontsource { family } => {
            manifest.dependency("topcoat", topcoat().features(["font-fontsource"]))?;
            let family = Family::by_id(family)
                .ok_or_else(|| format!("unknown Fontsource family `{family}`"))?;
            Some(FontInfo::new(family))
        }
    };

    let mut plan = ProjectPlan::default();

    // Only the default set has an icon name known to exist for the example.
    let iconify_set = match &options.icons {
        IconSetup::Iconify { set } => Some(set.as_str()),
        IconSetup::None | IconSetup::Custom => None,
    };
    let iconify_example = iconify_set == Some(EXAMPLE_ICON_SET);
    let custom_icons = options.icons == IconSetup::Custom;
    if custom_icons {
        plan.add(
            "src/icons.rs",
            include_str!("templates/icons/custom/src/icons.rs"),
        )?;
    }

    // A package name has no characters that need escaping, so its debug form is a
    // valid string literal.
    let title = format!("{:?}", name.as_str());
    let paths = Paths(options.routing);
    let classes = Classes::new(options.tailwind, options.ui);
    let ui = options.ui;

    // Topcoat UI: the theme as the Tailwind input and the components the pages use,
    // planned like `topcoat ui init` followed by `topcoat ui add`.
    if ui {
        manifest.dependency("topcoat", topcoat().features(["ui"]))?;
        let registry = Registry::embedded(topcoat_ui_registry::FILES)
            .map_err(|error| format!("failed to load the UI registry: {error}"))?;
        let scaffold = manage::scaffold(
            &registry,
            &ScaffoldOptions {
                theme: None,
                components: UI_COMPONENTS,
            },
        )?;
        for file in scaffold {
            let contents = if file.path == Path::new(STYLESHEET) {
                theme_with_font(&file.contents, font.as_ref())
            } else {
                file.contents
            };
            plan.add(file.path, contents)?;
        }
    }

    // The todo feature, stored with the selected database integration or in memory.
    plan.add(
        "src/features.rs",
        include_str!("templates/base/src/features.rs"),
    )?;
    if toasty {
        plan.add(
            "src/db.rs",
            include_str!("templates/storage/toasty/src/db.rs"),
        )?;
        plan.add(
            "src/features/todo.rs",
            include_str!("templates/storage/toasty/src/features/todo.rs"),
        )?;
    } else {
        plan.add(
            "src/features/todo.rs",
            include_str!("templates/storage/memory/src/features/todo.rs"),
        )?;
    }

    // The todo pages, built with the selected interaction approach.
    let (todos, todo_id) = match options.interaction {
        Interaction::None => (
            render(&FormsTodos { paths, classes, ui })?,
            Some(render(&FormsTodoId { paths })?),
        ),
        Interaction::AlpineAjax => (
            render(&AlpineAjaxTodos { paths, classes, ui })?,
            Some(render(&FormsTodoId { paths })?),
        ),
        Interaction::Htmx => (
            render(&HtmxTodos { paths, classes, ui })?,
            Some(render(&HtmxTodoId { paths })?),
        ),
        Interaction::Datastar => (
            render(&DatastarTodos { paths, classes, ui })?,
            Some(render(&DatastarTodoId { paths })?),
        ),
        Interaction::Topcoat => (render(&TopcoatTodos { paths, classes, ui })?, None),
    };
    plan.add("src/app/todos.rs", format_rust("src/app/todos.rs", &todos)?)?;
    if let Some(todo_id) = todo_id {
        plan.add(
            "src/app/todos/id.rs",
            format_rust("src/app/todos/id.rs", &todo_id)?,
        )?;
    }

    let app = render(&AppRs {
        title: &title,
        ui,
        routing: options.routing,
        discover: options.routing == Routing::Discover
            || (options.routing == Routing::Module && options.interaction == Interaction::Topcoat),
        paths,
        classes,
        interaction: options.interaction,
        toasty,
        tailwind: options.tailwind,
        iconify_example,
        icon_set_hint: iconify_set.filter(|_| !iconify_example),
        custom_icons,
        font,
    })?;
    plan.add("src/app.rs", format_rust("src/app.rs", &app)?)?;
    let main = render(&MainRs {
        toasty,
        ui,
        custom_icons,
    })?;
    plan.add("src/main.rs", format_rust("src/main.rs", &main)?)?;

    let build = BuildRs {
        tailwind: options.tailwind,
        icon_sets: options.icon_sets(),
    };
    if build.is_needed() {
        plan.add("build.rs", format_rust("build.rs", &render(&build)?)?)?;
    }

    // With Tailwind, the stylesheet is the Tailwind input. Topcoat UI supplies its theme
    // as the stylesheet instead.
    if !ui {
        let styles = if options.tailwind {
            include_str!("templates/tailwind/styles.css")
        } else {
            include_str!("templates/base/styles.css")
        };
        plan.add(STYLESHEET, styles)?;
    }

    plan.add("Cargo.toml", manifest.render())?;
    plan.add(".gitignore", render(&Gitignore { toasty })?)?;
    plan.add(
        "README.md",
        render(&Readme {
            name: name.as_str(),
            toasty,
        })?,
    )?;
    Ok(plan)
}

/// Points the UI theme's sans-serif font at the application's Fontsource family. The
/// theme is unchanged without a font integration, or if it does not declare its font as
/// expected.
fn theme_with_font(theme: &str, font: Option<&FontInfo>) -> String {
    match font {
        Some(font) => theme.replace(
            THEME_FONT,
            &format!("--font-sans: {:?}, {};", font.name, font.generic),
        ),
        None => theme.to_string(),
    }
}

/// The `topcoat` dependency without default features. Features are added per
/// integration and merged by the manifest.
fn topcoat() -> Dependency {
    Dependency::new(TOPCOAT_VERSION).no_default_features()
}

fn render(template: &impl Template) -> Result<String, String> {
    let mut output = template
        .render()
        .map_err(|error| format!("failed to render a template: {error}"))?;
    // Askama drops the final newline of every template file; restore it.
    output.push('\n');
    Ok(output)
}

/// Formats the Topcoat macro bodies in `source`.
fn format_rust(path: &str, source: &str) -> Result<String, String> {
    pretty_print_str(&format::registry(), source).map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        format!("failed to format {path}: {}", errors.join("; "))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::new::choice::DatabaseBackend;

    fn minimal() -> ProjectOptions {
        ProjectOptions {
            routing: Routing::Module,
            database: DatabaseSetup::None,
            interaction: Interaction::None,
            tailwind: false,
            icons: IconSetup::None,
            font: FontSetup::None,
            ui: false,
        }
    }

    fn file<'a>(plan: &'a ProjectPlan, path: &str) -> Option<&'a str> {
        plan.files()
            .find(|(file, _)| *file == Path::new(path))
            .map(|(_, contents)| contents)
    }

    fn features(dependency: &toml::Value) -> Vec<&str> {
        dependency["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|feature| feature.as_str().unwrap())
            .collect()
    }

    #[test]
    fn minimal_application_depends_only_on_the_features_it_uses() {
        let name = PackageName::new("my-app").unwrap();
        let plan = generate(&name, &minimal()).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert_eq!(manifest["package"]["name"].as_str(), Some("my-app"));
        let topcoat = &manifest["dependencies"]["topcoat"];
        assert_eq!(topcoat["version"].as_str(), Some(TOPCOAT_VERSION));
        assert_eq!(topcoat["default-features"].as_bool(), Some(false));
        assert!(!features(topcoat).contains(&"runtime"));
        assert!(!features(topcoat).contains(&"tailwind"));

        assert!(file(&plan, "src/app.rs").unwrap().contains("\"my-app\""));
        assert!(file(&plan, "build.rs").is_none());
        assert!(file(&plan, "src/icons.rs").is_none());
    }

    #[test]
    fn manual_routing_registers_handlers_without_discovery() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            routing: Routing::Manual,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(!features(&manifest["dependencies"]["topcoat"]).contains(&"discover"));
        let app = file(&plan, "src/app.rs").unwrap();
        assert!(app.contains(".page(home)"));
        assert!(app.contains("#[page(\"/\")]"));
        assert!(!app.contains("discover"));
    }

    #[test]
    fn iconify_stages_the_chosen_set() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            icons: IconSetup::Iconify {
                set: "tabler".to_string(),
            },
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"icon-iconify"));
        assert!(features(&manifest["build-dependencies"]["topcoat"]).contains(&"icon-iconify"));
        let build_script = file(&plan, "build.rs").unwrap();
        assert!(build_script.contains(".icon_set(\"tabler\")"));
        assert!(!build_script.contains("lucide"));
    }

    #[test]
    fn fontsource_fonts_are_registered_unless_discovered() {
        let name = PackageName::new("my-app").unwrap();
        for (routing, interaction) in [Routing::Module, Routing::Discover, Routing::Manual]
            .into_iter()
            .flat_map(|routing| {
                [Interaction::None, Interaction::Topcoat].map(|interaction| (routing, interaction))
            })
        {
            let options = ProjectOptions {
                routing,
                interaction,
                font: FontSetup::Fontsource {
                    family: "roboto".to_string(),
                },
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();

            let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
            assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"font-fontsource"));
            let app = file(&plan, "src/app.rs").unwrap();
            assert!(app.contains("fontsource_font!(ROBOTO, weight: [400, 700], style: Normal)"));
            assert!(app.contains("font::link(font: ROBOTO)"));
            // Registering a discovered font again would register its route twice.
            let registered = app.contains(".font(ROBOTO)");
            let discovered = app.contains(".discover()");
            assert_ne!(registered, discovered, "{routing:?} {interaction:?}");
        }
    }

    #[test]
    fn each_interaction_builds_the_todo_pages_its_own_way() {
        let name = PackageName::new("my-app").unwrap();
        for (interaction, feature) in [
            (Interaction::None, None),
            (Interaction::Topcoat, Some("runtime")),
            (Interaction::Htmx, Some("htmx")),
            (Interaction::Datastar, Some("datastar")),
            (Interaction::AlpineAjax, Some("alpine-ajax")),
        ] {
            let options = ProjectOptions {
                routing: Routing::Manual,
                interaction,
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();
            let label = format!("{interaction:?}");

            let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
            let dependencies = &manifest["dependencies"];
            if let Some(feature) = feature {
                assert!(
                    features(&dependencies["topcoat"]).contains(&feature),
                    "{label}"
                );
            }
            // The runtime calls procedures with plain arguments; every other approach
            // deserializes form or signal data.
            let runtime = interaction == Interaction::Topcoat;
            assert_eq!(dependencies.get("serde").is_none(), runtime, "{label}");

            assert!(file(&plan, "src/app/todos.rs").is_some(), "{label}");
            assert_eq!(
                file(&plan, "src/app/todos/id.rs").is_none(),
                runtime,
                "{label}"
            );
            let app = file(&plan, "src/app.rs").unwrap();
            assert!(app.contains(".page(todos::page)"), "{label}");
            if runtime {
                assert!(app.contains(".route(todos::todo_list)"), "{label}");
                assert!(app.contains(".route(todos::add)"), "{label}");
            } else {
                assert!(app.contains(".route(todos::create)"), "{label}");
                assert!(app.contains(".route(todos::id::toggle)"), "{label}");
            }
        }
    }

    #[test]
    fn todos_are_stored_with_toasty_or_in_memory() {
        let name = PackageName::new("my-app").unwrap();

        let plan = generate(&name, &minimal()).unwrap();
        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(manifest["dependencies"].get("toasty").is_none());
        assert!(file(&plan, "src/db.rs").is_none());
        assert!(
            !file(&plan, "src/features/todo.rs")
                .unwrap()
                .contains("toasty")
        );

        let options = ProjectOptions {
            database: DatabaseSetup::Toasty {
                backend: DatabaseBackend::Sqlite,
            },
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();
        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        let toasty = &manifest["dependencies"]["toasty"];
        assert!(features(toasty).contains(&"sqlite"));
        assert!(manifest["dependencies"].get("toasty-cli").is_some());
        assert!(file(&plan, "src/db.rs").is_some());
        assert!(
            file(&plan, "src/features/todo.rs")
                .unwrap()
                .contains("toasty::Model")
        );
        assert!(file(&plan, "src/main.rs").unwrap().contains("ToastyCli"));
        assert!(file(&plan, ".gitignore").unwrap().contains("data.db"));
    }

    #[test]
    fn browser_scripts_match_the_examples() {
        let name = PackageName::new("my-app").unwrap();
        for (interaction, example) in [
            (Interaction::Htmx, "htmx"),
            (Interaction::Datastar, "datastar"),
            (Interaction::AlpineAjax, "alpine-ajax"),
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples")
                .join(example)
                .join("src/main.rs");
            let source = std::fs::read_to_string(path).unwrap();
            let urls: Vec<&str> = source
                .split('"')
                .filter(|part| part.starts_with("https://cdn.jsdelivr.net/"))
                .collect();
            assert!(!urls.is_empty(), "{example}");

            let options = ProjectOptions {
                interaction,
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();
            let app = file(&plan, "src/app.rs").unwrap();
            for url in urls {
                assert!(app.contains(url), "{example}: {url}");
            }
        }
    }

    #[test]
    fn custom_icons_live_in_their_own_module() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            icons: IconSetup::Custom,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        assert!(file(&plan, "src/icons.rs").is_some());
        assert!(file(&plan, "src/main.rs").unwrap().contains("mod icons;"));
        assert!(file(&plan, "build.rs").is_none());
    }

    #[test]
    fn tailwind_builds_the_stylesheet_from_styles_css() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            tailwind: true,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"tailwind"));
        let build = &manifest["build-dependencies"]["topcoat"];
        assert_eq!(build["default-features"].as_bool(), Some(false));
        assert!(features(build).contains(&"tailwind"));

        let build_script = file(&plan, "build.rs").unwrap();
        assert!(build_script.contains("\"styles.css\""));
        assert!(file(&plan, "styles.css").unwrap().contains("tailwindcss"));
        assert!(
            file(&plan, "src/app.rs")
                .unwrap()
                .contains("tailwind::stylesheet!")
        );
    }

    fn ui(font: &str) -> ProjectOptions {
        ProjectOptions {
            tailwind: true,
            icons: IconSetup::Iconify {
                set: EXAMPLE_ICON_SET.to_string(),
            },
            font: FontSetup::Fontsource {
                family: font.to_string(),
            },
            ui: true,
            ..minimal()
        }
    }

    #[test]
    fn ui_installs_its_theme_and_the_components_the_pages_use() {
        let name = PackageName::new("my-app").unwrap();
        let plan = generate(&name, &ui("geist")).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"ui"));

        let theme = file(&plan, STYLESHEET).unwrap();
        assert!(theme.contains(THEME_FONT));
        assert!(file(&plan, "components.toml").is_some());
        let modules = file(&plan, "src/components.rs").unwrap();
        for component in UI_COMPONENTS {
            assert!(
                modules.contains(&format!("pub mod {component};")),
                "{component}"
            );
            let path = format!("src/components/{component}.rs");
            assert!(file(&plan, &path).is_some(), "{component}");
        }
        assert!(
            file(&plan, "src/main.rs")
                .unwrap()
                .contains("mod components;")
        );
        assert!(file(&plan, "src/app.rs").unwrap().contains("card("));
        assert!(file(&plan, "src/app/todos.rs").unwrap().contains("button("));
    }

    #[test]
    fn ui_theme_uses_the_chosen_font_family() {
        let name = PackageName::new("my-app").unwrap();
        let plan = generate(&name, &ui("roboto")).unwrap();

        let theme = file(&plan, STYLESHEET).unwrap();
        assert!(!theme.contains(THEME_FONT));
        assert!(theme.contains("\"Roboto\""));
    }

    #[test]
    fn default_ui_theme_declares_the_expected_font() {
        let registry = Registry::embedded(topcoat_ui_registry::FILES).unwrap();
        let name = registry.theme_names().next().unwrap().to_string();
        let source = registry.theme(&name).unwrap().read_source().unwrap();
        assert!(source.contains(THEME_FONT));
    }

    #[test]
    fn dependency_versions_match_the_workspace() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        let workspace: toml::Table = std::fs::read_to_string(path).unwrap().parse().unwrap();
        for (name, expected) in [("tokio", TOKIO_VERSION), ("serde", SERDE_VERSION)] {
            let dependency = &workspace["workspace"]["dependencies"][name];
            let version = dependency
                .as_str()
                .or_else(|| dependency["version"].as_str());
            assert_eq!(version, Some(expected), "{name}");
        }
    }
}
