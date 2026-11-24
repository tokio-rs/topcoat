use std::collections::BTreeSet;

use super::{
    package::Package,
    state::{InstallState, InstalledTheme, RegistryState},
    workspace::Workspace,
};
use crate::Registry;

/// A registry name and its component statuses or load error.
pub struct RegistryListing {
    pub name: String,
    /// The status of the installed theme, if it was installed from this registry.
    pub theme: Option<ThemeStatus>,
    pub outcome: Result<Vec<ComponentStatus>, String>,
}

/// The installed theme's name and its install status, which is never
/// [`InstallStatus::Available`].
pub struct ThemeStatus {
    pub name: String,
    pub status: InstallStatus,
}

/// A component's name and its install status within a registry.
pub struct ComponentStatus {
    pub name: String,
    pub status: InstallStatus,
}

/// The install status of a registry component or theme in the package.
pub enum InstallStatus {
    /// Offered by the registry but not installed; carries the latest hash.
    Available { hash: String },
    /// Installed at the hash the registry currently offers.
    UpToDate { hash: String },
    /// Installed at a different hash than the registry now offers.
    Update { installed: String, latest: String },
    /// Tracked as installed under this registry, which no longer offers it.
    Orphaned { installed: String },
}

/// Lists available registries and the install status of their components.
///
/// Includes registries recorded in the install state even if they are no longer
/// dependencies. Pass `selected` to list just one registry. A component is installed
/// only under the registry recorded for it, and so is the theme. An unavailable
/// registry produces orphaned statuses for the tracked theme and components, and a
/// load error if no components are tracked.
///
/// # Errors
///
/// Returns an error if the install state or workspace cannot be loaded, or if
/// `selected` names a registry that is neither a dependency nor tracked in the
/// install state.
pub fn list(package: &Package, selected: Option<&str>) -> Result<Vec<RegistryListing>, String> {
    let state = InstallState::load(package)?;
    let workspace = Workspace::load(package)?;

    // The registries worth listing: discoverable dependency registries plus any
    // still tracked in the install state.
    let names: BTreeSet<String> = workspace
        .available_registries()
        .into_iter()
        .chain(state.registries.keys().cloned())
        .chain(state.theme.iter().map(|theme| theme.registry.clone()))
        .collect();

    if let Some(name) = selected
        && !names.contains(name)
    {
        return Err(format!("unknown registry `{name}`"));
    }

    let empty = RegistryState::default();
    let listings = names
        .iter()
        .filter(|name| selected.is_none_or(|chosen| chosen == name.as_str()))
        .map(|name| {
            let tracked = state.registries.get(name).unwrap_or(&empty);
            let theme = state.theme.as_ref().filter(|theme| theme.registry == *name);
            listing_for(&workspace, name, tracked, theme)
        })
        .collect();

    Ok(listings)
}

/// Loads a registry and classifies its components and the `theme` installed from it.
/// If loading fails, reports the theme and tracked components as orphaned, and returns
/// the load error when no components are tracked.
fn listing_for(
    workspace: &Workspace,
    name: &str,
    state: &RegistryState,
    theme: Option<&InstalledTheme>,
) -> RegistryListing {
    let registry = workspace.registry_dir(name).and_then(|dir| {
        Registry::load(dir).map_err(|error| format!("failed to load registry `{name}`: {error}"))
    });
    let (theme, outcome) = match registry {
        Ok(registry) => match theme
            .map(|theme| theme_status(Some(&registry), theme))
            .transpose()
        {
            Ok(theme) => (theme, statuses(&registry, state)),
            Err(error) => (None, Err(error)),
        },
        Err(error) => (
            theme.and_then(|theme| theme_status(None, theme).ok()),
            if state.components.is_empty() {
                Err(error)
            } else {
                Ok(orphaned(state))
            },
        ),
    };
    RegistryListing {
        name: name.to_string(),
        theme,
        outcome,
    }
}

/// Compares the hash recorded for the installed theme with the registry's current
/// one. The theme is orphaned if `registry` is `None` or no longer offers it. Fails if
/// the theme's source cannot be read.
fn theme_status(
    registry: Option<&Registry>,
    installed: &InstalledTheme,
) -> Result<ThemeStatus, String> {
    let status = match registry.and_then(|registry| registry.theme(&installed.name)) {
        None => InstallStatus::Orphaned {
            installed: installed.hash.clone(),
        },
        Some(theme) => {
            let latest = theme
                .hash()
                .map_err(|error| format!("failed to hash theme `{}`: {error}", installed.name))?;
            if installed.hash == latest {
                InstallStatus::UpToDate { hash: latest }
            } else {
                InstallStatus::Update {
                    installed: installed.hash.clone(),
                    latest,
                }
            }
        }
    };
    Ok(ThemeStatus {
        name: installed.name.clone(),
        status,
    })
}

/// Compares current component hashes with install-state hashes, including installed
/// components no longer offered by the registry. Fails if a source cannot be read.
fn statuses(registry: &Registry, state: &RegistryState) -> Result<Vec<ComponentStatus>, String> {
    let names: Vec<&str> = registry.names().collect();
    let mut out = Vec::new();

    for component_name in &names {
        let component = registry
            .get(component_name)
            .expect("name came from the registry");
        let latest = component
            .hash()
            .map_err(|error| format!("failed to hash component `{component_name}`: {error}"))?;
        let status = match state.components.get(*component_name) {
            None => InstallStatus::Available { hash: latest },
            Some(installed) if installed.hash == latest => InstallStatus::UpToDate { hash: latest },
            Some(installed) => InstallStatus::Update {
                installed: installed.hash.clone(),
                latest,
            },
        };
        out.push(ComponentStatus {
            name: (*component_name).to_string(),
            status,
        });
    }

    for (component_name, installed) in &state.components {
        if !names.contains(&component_name.as_str()) {
            out.push(ComponentStatus {
                name: component_name.clone(),
                status: InstallStatus::Orphaned {
                    installed: installed.hash.clone(),
                },
            });
        }
    }

    Ok(out)
}

/// Reports tracked components as orphaned when their registry cannot be loaded.
fn orphaned(state: &RegistryState) -> Vec<ComponentStatus> {
    state
        .components
        .iter()
        .map(|(name, installed)| ComponentStatus {
            name: name.clone(),
            status: InstallStatus::Orphaned {
                installed: installed.hash.clone(),
            },
        })
        .collect()
}
