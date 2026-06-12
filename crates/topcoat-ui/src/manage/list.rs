use crate::{Registry, Source};

use super::project::Project;
use super::state::{InstallState, RegistryState};

/// One registry's listing: its name and location, and either the status of its
/// components or the error encountered loading it.
pub struct RegistryListing {
    pub name: String,
    pub url: String,
    pub outcome: Result<Vec<ComponentStatus>, String>,
}

/// A component's name and its install status within a registry.
pub struct ComponentStatus {
    pub name: String,
    pub status: InstallStatus,
}

/// How a component relates to what the project has installed from its registry.
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
/// A component counts as installed only when it is tracked under *that*
/// registry, so the same name installed from a different registry is not treated
/// as installed here. With `selected`, only that registry is listed. Failures to
/// load an individual registry are reported per registry (in `outcome`) rather
/// than failing the whole listing.
pub async fn list(
    project: &Project,
    selected: Option<&str>,
) -> Result<Vec<RegistryListing>, String> {
    let mut state = InstallState::load(project)?;

    // Ensure there is something to list: a named registry that isn't tracked yet
    // (only valid for one with a built-in location), or the project's default
    // registry when nothing has been added yet.
    match selected {
        Some(name) if !state.registries.contains_key(name) => {
            let url = InstallState::default_url(name)
                .ok_or_else(|| format!("unknown registry `{name}`"))?;
            let registry = RegistryState::new(name, url, &state.base_dir);
            state.registries.insert(name.to_string(), registry);
        }
        None if state.registries.is_empty() => {
            let name = state.default_registry.clone();
            let url = InstallState::default_url(&name).ok_or_else(|| {
                format!("default registry `{name}` has no known location; run `topcoat ui add` first")
            })?;
            let registry = RegistryState::new(&name, url, &state.base_dir);
            state.registries.insert(name, registry);
        }
        _ => {}
    }

    let mut listings = Vec::new();
    for (name, registry_state) in &state.registries {
        if selected.is_some_and(|chosen| chosen != name) {
            continue;
        }
        listings.push(listing_for(project, name, registry_state).await);
    }

    Ok(listings)
}

/// Builds one registry's listing, loading it and classifying its components.
async fn listing_for(project: &Project, name: &str, state: &RegistryState) -> RegistryListing {
    let working_url = project.to_working(&state.url);
    let outcome = match Registry::load(Source::parse(&working_url)).await {
        Ok(registry) => Ok(statuses(&registry, state)),
        Err(error) => Err(error.to_string()),
    };
    RegistryListing {
        name: name.to_string(),
        url: state.url.clone(),
        outcome,
    }
}

/// Classifies every component a registry offers, plus any tracked under it that
/// it no longer offers.
fn statuses(registry: &Registry, state: &RegistryState) -> Vec<ComponentStatus> {
    let names: Vec<&str> = registry.names().collect();
    let mut out = Vec::new();

    for component_name in &names {
        let component = registry
            .get(component_name)
            .expect("name came from the registry");
        let latest = component.hash().to_string();
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

    out
}
