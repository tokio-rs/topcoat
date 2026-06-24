use std::collections::BTreeSet;

use crate::Registry;

use super::package::Package;
use super::state::{InstallState, RegistryState};
use super::workspace::Workspace;

/// One registry's listing: its crate name, and either the status of its
/// components or the error encountered loading it.
pub struct RegistryListing {
    pub name: String,
    pub outcome: Result<Vec<ComponentStatus>, String>,
}

/// A component's name and its install status within a registry.
pub struct ComponentStatus {
    pub name: String,
    pub status: InstallStatus,
}

/// How a component relates to what the package has installed from its registry.
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

/// Lists registries and the install status of their components.
///
/// The registries listed are those the package can add from (the default plus
/// any dependency registry), together with any registry still tracked in the
/// install state (so components from a since-removed dependency are not hidden).
/// A component counts as installed only when it is tracked under *that* registry.
/// With `selected`, only that registry is listed. Failures to load an individual
/// registry are reported per registry (in `outcome`) rather than failing the
/// whole listing.
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

/// Builds one registry's listing, resolving and loading it and classifying its
/// components. When the registry cannot be resolved or loaded, any components
/// still tracked under it are reported as orphaned rather than losing them to a
/// bare load error.
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

/// Classifies every component a registry offers, plus any tracked under it that
/// it no longer offers. Each offered component's source is read and hashed to
/// learn its current version; a failure to do so fails the whole listing.
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

/// Reports every tracked component as orphaned, used when the registry itself
/// can no longer be loaded.
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
