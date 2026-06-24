use std::path::PathBuf;

use crate::{DEFAULT_REGISTRY_CRATE, Registry, content_hash};

use super::ChooseTheme;
use super::package::Package;
use super::state::{InstallState, InstalledTheme, STATE_FILE};
use super::workspace::Workspace;

/// How to set up a package's install state.
pub struct InitOptions {
    /// Base directory for component install output (default `src/components`).
    pub components_dir: Option<PathBuf>,
    /// The theme to install by name. When `None`, the sole registered theme is
    /// installed, or the user is asked to choose when several are offered; a
    /// theme is always installed, never skipped.
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

/// Sets up a package's initial install state, which the other commands (`add`,
/// `remove`, `list`) require before they will run.
///
/// Besides fixing where components install, init always installs a theme: the
/// chosen theme's CSS is copied into the package as its Tailwind input, and the
/// theme is recorded in the install state. The theme is named by
/// [`InitOptions::theme`], or chosen via `choose` when none is given. Errors if
/// the package is already initialized rather than clobbering its state.
///
/// # Errors
///
/// Returns an error if the package is already initialized, the default
/// registry cannot be loaded or offers no themes, a named theme is unknown,
/// a theme selection prompt is declined, or writing the stylesheet or install
/// state fails.
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

/// Resolves the package's theme without writing anything. A theme is mandatory,
/// so this either produces a plan or fails: the default registry must be
/// reachable and offer at least one theme, and an explicitly named theme must
/// exist. When no theme was named, the sole offered theme is taken, or `choose`
/// picks one when the registry offers several.
fn plan_theme(
    package: &Package,
    requested: Option<&str>,
    choose: &mut ChooseTheme<'_>,
) -> Result<ThemePlan, String> {
    let registry_crate = DEFAULT_REGISTRY_CRATE;
    let workspace = Workspace::load(package)?;
    let dir = workspace
        .registry_dir(registry_crate)
        .map_err(|error| format!("cannot install a theme: {error}"))?;
    let registry = Registry::load(dir)
        .map_err(|error| format!("failed to load registry `{registry_crate}`: {error}"))?;

    let names: Vec<String> = registry.theme_names().map(str::to_string).collect();
    if names.is_empty() {
        return Err(format!(
            "registry `{registry_crate}` offers no themes to install"
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
        registry: registry_crate.to_string(),
        hash: content_hash(&contents),
        // Installed at the package root by default, alongside `components.toml`.
        file: PathBuf::from(theme.file_name()),
        contents,
    })
}
