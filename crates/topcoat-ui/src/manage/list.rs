use std::collections::BTreeSet;

use super::{
    package::Package,
    state::{InstallState, RegistryState},
    workspace::Workspace,
};
use crate::Registry;

/// The listing of one registry, returned by [`list`].
pub struct RegistryListing {
    /// The name of the registry.
    pub name: String,
    /// The status of each component, or the error from loading the registry.
    pub outcome: Result<Vec<ComponentStatus>, String>,
}

/// The install status of one component in a [`RegistryListing`].
pub struct ComponentStatus {
    /// The name of the component.
    pub name: String,
    /// Whether and at which version the component is installed.
    pub status: InstallStatus,
}

/// Whether a component is installed, compared with what its registry offers.
///
/// The hashes are [`content_hash`](crate::content_hash) values.
pub enum InstallStatus {
    /// The registry offers the component, but it is not installed.
    Available {
        /// The hash of the registry's source.
        hash: String,
    },
    /// The component is installed with the registry's current source.
    UpToDate {
        /// The hash of the installed and the registry's source.
        hash: String,
    },
    /// The component is installed, but the registry's source has changed
    /// since.
    Update {
        /// The hash recorded when the component was installed.
        installed: String,
        /// The hash of the registry's current source.
        latest: String,
    },
    /// The component is installed from this registry, but the registry no
    /// longer offers it or can no longer be loaded.
    Orphaned {
        /// The hash recorded when the component was installed.
        installed: String,
    },
}

/// Lists registries and the install status of their components: implements
/// `topcoat ui list`.
///
/// This lists every registry the package can add from, plus every registry
/// that `components.toml` still records components from, sorted by name. A
/// component counts as installed only if it was installed from that same
/// registry. With `selected`, only that registry is listed. An error while
/// loading one registry is reported in its [`RegistryListing::outcome`] and
/// does not fail the whole listing.
///
/// The hashes compare the registry's source only. Local edits to an installed
/// file do not change its status.
///
/// # Errors
///
/// Returns an error if the package has no `components.toml`, if
/// `cargo metadata` fails, or if `selected` names an unknown registry.
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
