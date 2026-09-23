use std::path::PathBuf;

use super::{
    ChooseTheme,
    package::Package,
    state::{InstallState, InstalledTheme, STATE_FILE},
    workspace::Workspace,
};
use crate::{DEFAULT_REGISTRY, Registry, content_hash};

/// Options for [`init`].
pub struct InitOptions {
    /// The directory components are installed into, relative to the package
    /// root. Defaults to `src/components`.
    pub components_dir: Option<PathBuf>,
    /// The name of the theme to install. When `None` and the built-in registry
    /// offers only one theme, that theme is installed. When it offers several,
    /// the [`ChooseTheme`] callback picks one. A theme is always installed.
    pub theme: Option<String>,
}

/// The theme installed by [`init`].
pub struct InstalledThemeInfo {
    /// The name of the theme, such as `neutral`.
    pub name: String,
    /// The name of the registry it came from.
    pub registry: String,
    /// The path of the written stylesheet, relative to the package root.
    pub file: PathBuf,
}

/// The result of [`init`].
pub struct Initialized {
    /// The path of the created `components.toml`, relative to the package
    /// root.
    pub state_file: PathBuf,
    /// The directory components are installed into, relative to the package
    /// root.
    pub components_dir: PathBuf,
    /// The theme that was installed.
    pub theme: InstalledThemeInfo,
}

/// Sets up a package for `topcoat ui`: implements `topcoat ui init`.
///
/// This creates `components.toml` at the package root, which [`add`](super::add()),
/// [`list`](super::list()), and [`remove`](super::remove()) require. It records
/// the directory components are installed into.
///
/// It also installs a theme from the built-in registry: the theme's CSS is
/// written to `styles.css` at the package root, to be used as the Tailwind
/// input, and the theme is recorded in `components.toml`. The theme is the one
/// named in [`InitOptions::theme`], or the only one on offer, or the one that
/// `choose` picks.
///
/// Nothing is written if the theme cannot be resolved.
///
/// # Errors
///
/// Returns an error if the package already has a `components.toml`, if the
/// built-in registry cannot be loaded or has no themes, if the named theme
/// does not exist, if `choose` returns an error, or if a file cannot be
/// written.
///
/// # Panics
///
/// Panics if `choose` returns a name that was not offered to it.
#[track_caller]
pub fn init(
    package: &Package,
    options: InitOptions,
    choose: &mut ChooseTheme<'_>,
) -> Result<Initialized, String> {
    // Resolve the theme (load the registry, prompt if needed, read the source)
    // before touching disk, so an unreachable registry or a bad theme name (or a
    // declined prompt) leaves the package untouched rather than half-initialized.
    // The already-initialized check happens up front, before any prompt.
    if package.state_path().exists() {
        return Err(format!(
            "{} already exists; the package is already initialized",
            package.state_path().display()
        ));
    }
    let theme = plan_theme(package, options.theme.as_deref(), choose)?;

    // Commit: write the stylesheet, then create and record the install state.
    let file = package.resolve(&theme.file);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    std::fs::write(&file, &theme.contents)
        .map_err(|error| format!("failed to write {}: {error}", file.display()))?;

    let mut state = InstallState::create(package, options.components_dir)?;
    state.theme = Some(InstalledTheme {
        name: theme.name.clone(),
        registry: theme.registry.clone(),
        hash: theme.hash,
        file: theme.file.clone(),
    });
    state.save(package)?;

    Ok(Initialized {
        state_file: PathBuf::from(STATE_FILE),
        components_dir: state.components_dir.clone(),
        theme: InstalledThemeInfo {
            name: theme.name,
            registry: theme.registry,
            file: theme.file,
        },
    })
}

/// A theme resolved and read but not yet written.
struct ThemePlan {
    name: String,
    registry: String,
    hash: String,
    /// Package-relative destination of the stylesheet.
    file: PathBuf,
    /// The stylesheet's CSS, read from the registry.
    contents: String,
}

/// Resolves the package's theme without writing anything. A theme is required,
/// so this either returns a plan or fails: the default registry must load and
/// offer at least one theme, and a named theme must exist. When no theme was
/// named, the only offered theme is taken, or `choose` picks one when the
/// registry offers several.
#[track_caller]
fn plan_theme(
    package: &Package,
    requested: Option<&str>,
    choose: &mut ChooseTheme<'_>,
) -> Result<ThemePlan, String> {
    let registry_name = DEFAULT_REGISTRY;
    let workspace = Workspace::load(package)?;
    let dir = workspace
        .registry_dir(registry_name)
        .map_err(|error| format!("cannot install a theme: {error}"))?;
    let registry = Registry::load(dir)
        .map_err(|error| format!("failed to load registry `{registry_name}`: {error}"))?;

    let names: Vec<String> = registry.theme_names().map(str::to_string).collect();
    if names.is_empty() {
        return Err(format!(
            "registry `{registry_name}` offers no themes to install"
        ));
    }

    let chosen = match requested {
        Some(name) if names.iter().any(|known| known == name) => name.to_string(),
        Some(name) => {
            return Err(format!(
                "unknown theme `{name}`; available: {}",
                names.join(", ")
            ));
        }
        // With a single theme on offer there is nothing to choose, so install it
        // without prompting; the picker only appears once the registry ships more.
        None if names.len() == 1 => names[0].clone(),
        None => choose(&names)?,
    };

    let theme = registry
        .theme(&chosen)
        .expect("chosen theme came from the registry");
    let contents = theme
        .read_source()
        .map_err(|error| format!("failed to read theme `{chosen}`: {error}"))?;

    Ok(ThemePlan {
        name: chosen,
        registry: registry_name.to_string(),
        hash: content_hash(&contents),
        // Installed at the package root by default, alongside `components.toml`.
        file: PathBuf::from(theme.file_name()),
        contents,
    })
}
