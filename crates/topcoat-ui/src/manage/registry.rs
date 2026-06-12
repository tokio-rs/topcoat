use std::path::PathBuf;

use crate::{Registry, Source};

use super::project::Project;
use super::state::{InstallState, RegistryState};

/// A registry added to the install state by [`add_registry`].
pub struct AddedRegistry {
    /// The name the registry is tracked under.
    pub name: String,
    /// The stored registry location (a path, `file://` path, or `http(s)://` URL).
    pub url: String,
    /// Where this registry's components will be installed.
    pub components_dir: PathBuf,
}

/// A registry removed from the install state by [`remove_registry`].
pub struct RemovedRegistry {
    /// The name the registry was tracked under.
    pub name: String,
    /// The stored registry location it had.
    pub url: String,
    /// Whether it had been the default registry, now leaving the project with
    /// none.
    pub was_default: bool,
}

/// Adds a registry to the project's install state.
///
/// The registry at `url` is loaded both to confirm it is reachable and to learn
/// the name it declares for itself, used as the tracked name unless `name`
/// overrides it. Adding a registry whose name or location is already tracked is
/// an error, so the call never silently shadows an existing registry. Its
/// components install under `<base_dir>/<name>`, fixed at this point.
pub async fn add_registry(
    project: &Project,
    url: &str,
    name: Option<&str>,
) -> Result<AddedRegistry, String> {
    let mut state = InstallState::load(project)?;

    let stored = project.to_stored(url);
    let working = project.to_working(&stored);

    // Load the registry to validate it is reachable and to learn its declared
    // name, used unless the caller overrides it.
    let registry = Registry::load(Source::parse(&working))
        .await
        .map_err(|error| format!("failed to load registry {working}: {error}"))?;
    let name = name.unwrap_or_else(|| registry.name()).to_string();

    if state.registries.contains_key(&name) {
        return Err(format!("registry `{name}` already exists"));
    }
    if let Some((existing, _)) = state.registries.iter().find(|(_, r)| r.url == stored) {
        return Err(format!(
            "registry at {stored} is already added as `{existing}`"
        ));
    }

    let registry_state = RegistryState::new(&name, stored.clone(), &state.base_dir);
    let components_dir = registry_state.components_dir.clone();
    state.registries.insert(name.clone(), registry_state);
    state.save(project)?;

    Ok(AddedRegistry {
        name,
        url: stored,
        components_dir,
    })
}

/// Removes a registry from the project's install state.
///
/// A registry that still has components installed cannot be removed — those
/// components must be removed first — so the registry never disappears out from
/// under tracked files. Removing the default registry is allowed; it simply
/// leaves the project with no default, after which `add` requires an explicit
/// `--registry` until a new default is set.
pub fn remove_registry(project: &Project, name: &str) -> Result<RemovedRegistry, String> {
    let mut state = InstallState::load(project)?;

    let registry = state
        .registries
        .get(name)
        .ok_or_else(|| format!("unknown registry `{name}`"))?;

    if !registry.components.is_empty() {
        let count = registry.components.len();
        let plural = if count == 1 { "" } else { "s" };
        return Err(format!(
            "registry `{name}` still has {count} component{plural} installed; remove them first"
        ));
    }

    let removed = state
        .registries
        .remove(name)
        .expect("registry resolved above");
    // Clear the default if it pointed at the registry just removed.
    let was_default = state.default_registry.as_deref() == Some(name);
    if was_default {
        state.default_registry = None;
    }
    state.save(project)?;

    Ok(RemovedRegistry {
        name: name.to_string(),
        url: removed.url,
        was_default,
    })
}

/// Sets a tracked registry as the project's default, used by `add` when no
/// `--registry` is given. The registry must already be tracked.
pub fn set_default(project: &Project, name: &str) -> Result<(), String> {
    let mut state = InstallState::load(project)?;

    if !state.registries.contains_key(name) {
        return Err(format!(
            "unknown registry `{name}`; add it with `topcoat ui registry add <url>`"
        ));
    }
    if state.default_registry.as_deref() == Some(name) {
        return Err(format!("registry `{name}` is already the default"));
    }

    state.default_registry = Some(name.to_string());
    state.save(project)
}
