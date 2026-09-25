use std::{io::ErrorKind, path::PathBuf};

use super::{module, package::Package, state::InstallState};

/// A component removed by [`remove`].
pub struct Removed {
    /// The component's name.
    pub name: String,
    /// The package-relative path of the deleted file.
    pub file: PathBuf,
    /// The registry it had been added from.
    pub registry: String,
}

/// Removes installed components.
///
/// Resolves every requested component before deleting files. Use `registry` to select
/// the source registry when a component name is installed from several registries.
/// Saves the install state after all removals. An I/O failure can leave some removals
/// complete and others unfinished.
///
/// # Errors
///
/// Returns an error if the install state cannot be loaded, a component's
/// registry cannot be resolved (unknown name, ambiguous across registries),
/// a file deletion fails for a reason other than the file already being gone,
/// a module declaration cannot be updated, or the state cannot be saved.
///
/// # Panics
///
/// Panics if a registry resolved during the up-front resolution phase is no
/// longer present in the install state when its component is deleted. This is
/// an internal invariant: resolution inserts the target pair, so the registry
/// must still be tracked.
#[track_caller]
pub fn remove(
    package: &Package,
    components: &[String],
    registry: Option<&str>,
) -> Result<Vec<Removed>, String> {
    let mut state = InstallState::load(package)?;

    // Resolve every component up front so a typo or ambiguity fails before any
    // file is deleted.
    let mut targets: Vec<(String, String)> = Vec::new();
    for component in components {
        let registry_name = resolve_registry(component, registry, &state)?;
        targets.push((registry_name, component.clone()));
    }

    // All registries install into one flat directory.
    let components_dir = state.components_dir.clone();

    // Reject an ambiguous module layout up front, before deleting any file.
    module::check(&package.resolve(&components_dir))?;

    let mut removed = Vec::new();
    for (registry_name, component) in targets {
        let registry = state
            .registries
            .get_mut(&registry_name)
            .expect("registry resolved above");
        // `None` means the component was listed more than once and is already
        // gone; skip it rather than reporting it twice.
        let Some(installed) = registry.components.remove(&component) else {
            continue;
        };

        let file = package.resolve(&installed.file);
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("failed to remove {}: {error}", file.display()));
            }
        }

        if let Some(file_name) = installed.file.file_name().and_then(|name| name.to_str()) {
            module::undeclare(&package.resolve(&components_dir), file_name)?;
        }

        // Drop the registry entry once its last component is gone, so the state
        // file doesn't keep an empty `[registries.<name>.components]` section.
        if registry.components.is_empty() {
            state.registries.remove(&registry_name);
        }

        removed.push(Removed {
            name: component,
            file: installed.file,
            registry: registry_name,
        });
    }

    state.save(package)?;

    Ok(removed)
}

/// Selects the named registry, or the sole registry from which the component is
/// installed.
fn resolve_registry(
    component: &str,
    registry: Option<&str>,
    state: &InstallState,
) -> Result<String, String> {
    if let Some(name) = registry {
        let registry = state
            .registries
            .get(name)
            .ok_or_else(|| format!("unknown registry `{name}`"))?;
        if !registry.components.contains_key(component) {
            return Err(format!(
                "component `{component}` is not installed from registry `{name}`"
            ));
        }
        return Ok(name.to_string());
    }

    let matches: Vec<&String> = state
        .registries
        .iter()
        .filter(|(_, registry)| registry.components.contains_key(component))
        .map(|(name, _)| name)
        .collect();

    match matches.as_slice() {
        [] => Err(format!("component `{component}` is not installed")),
        [name] => Ok((*name).clone()),
        many => Err(format!(
            "component `{component}` is installed from multiple registries ({}); pass --registry to choose",
            many.iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}
