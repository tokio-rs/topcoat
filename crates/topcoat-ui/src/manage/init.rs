use std::path::PathBuf;

use super::{
    ChooseTheme,
    package::Package,
    state::{InstallState, InstalledTheme, STATE_FILE},
    workspace::Workspace,
};
use crate::{DEFAULT_REGISTRY, Registry, content_hash};

/// Options for initializing component installation in a package.
pub struct InitOptions {
    /// Base directory for component install output (default `src/components`).
    pub components_dir: Option<PathBuf>,
    /// The theme to install. If omitted, use the sole available theme or ask the caller
    /// to choose. A theme is required.
    pub theme: Option<String>,
}

/// The theme installed by [`init`].
pub struct InstalledThemeInfo {
    /// The theme's name, e.g. `neutral`.
    pub name: String,
    /// The registry crate it came from.
    pub registry: String,
    /// The package-relative path of the written stylesheet.
    pub file: PathBuf,
}

/// The result of [`init`].
pub struct Initialized {
    /// The package-relative path of the created install-state file.
    pub state_file: PathBuf,
    /// The base directory recorded for component install output.
    pub components_dir: PathBuf,
    /// The theme that was installed.
    pub theme: InstalledThemeInfo,
}

/// Creates a package's install state and installs a theme.
///
/// Choose the component directory and theme through `InitOptions`. If no theme is named
/// and several are available, `choose` selects one. The theme CSS becomes the package's
/// Tailwind input. An initialized package cannot be initialized again.
///
/// # Errors
///
/// Returns an error if the package is already initialized, the default
/// registry cannot be loaded or offers no themes, a named theme is unknown,
/// a theme selection prompt is declined, or writing the stylesheet or install
/// state fails.
///
/// # Panics
///
/// Panics if `choose` returns a name that was not among the ones it was
/// offered.
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

/// Reads the selected theme and plans its destination without writing files. Uses the
/// sole theme if none is named, or calls `choose` when several are available.
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
