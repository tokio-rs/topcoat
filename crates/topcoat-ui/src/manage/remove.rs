use std::io::ErrorKind;
use std::path::PathBuf;

use super::module;
use super::package::Package;
use super::state::InstallState;

/// A component removed by [`remove`].
pub struct Removed {
    /// The component's name.
    pub name: String,
    /// The package-relative path of the deleted file.
    pub file: PathBuf,
    /// The registry it had been added from.
    pub registry: String,
}

/// Removes previously added components from the package.
///
/// Each component's registry is resolved first, so a bad name aborts before
/// anything is deleted: with `registry` it is removed from that registry,
/// otherwise from the sole registry it is installed from (an error if it is
/// installed from several). The state is saved once, after all removals.
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

/// Determines which registry the component should be removed from: the one named
/// via `registry`, or the sole registry that has it installed.
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
