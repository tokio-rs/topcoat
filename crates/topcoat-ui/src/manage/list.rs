use std::collections::BTreeSet;

use super::{
    package::Package,
    state::{InstallState, RegistryState},
    workspace::Workspace,
};
use crate::Registry;

/// A registry name and its component statuses or load error.
pub struct RegistryListing {
    pub name: String,
    pub outcome: Result<Vec<ComponentStatus>, String>,
}

/// A component's name and its install status within a registry.
pub struct ComponentStatus {
    pub name: String,
    pub status: InstallStatus,
}

/// The install status of a registry component in the package.
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
/// only under the registry recorded for it. An unavailable registry produces orphaned
/// statuses for tracked components, or a load error if none are tracked.
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
            listing_for(&workspace, name, tracked)
        })
        .collect();

    Ok(listings)
}

/// Loads a registry and classifies its components. If loading fails, reports tracked
/// components as orphaned, or returns the load error when none are tracked.
fn listing_for(workspace: &Workspace, name: &str, state: &RegistryState) -> RegistryListing {
    let outcome = match workspace.registry_dir(name).and_then(|dir| {
        Registry::load(dir).map_err(|error| format!("failed to load registry `{name}`: {error}"))
    }) {
        Ok(registry) => statuses(&registry, state),
        Err(error) if state.components.is_empty() => Err(error),
        Err(_) => Ok(orphaned(state)),
    };
    RegistryListing {
        name: name.to_string(),
        outcome,
    }
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
