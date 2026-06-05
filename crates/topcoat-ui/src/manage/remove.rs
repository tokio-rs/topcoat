use std::io::ErrorKind;
use std::path::PathBuf;

use super::module;
use super::project::Project;
use super::state::InstallState;

/// A component removed by [`remove`].
pub struct Removed {
    /// The project-relative path of the deleted file.
    pub file: PathBuf,
    /// The registry it had been added from.
    pub registry: String,
}

/// Removes a previously added component from the project.
///
/// With `registry`, the component is removed from that registry; otherwise the
/// sole registry it is installed from is used (an error if it is installed from
/// several).
pub fn remove(
    project: &Project,
    component: &str,
    registry: Option<&str>,
) -> Result<Removed, String> {
    let mut state = InstallState::load(project)?;

    let registry_name = resolve_registry(component, registry, &state)?;

    let registry = state
        .registries
        .get_mut(&registry_name)
        .expect("registry resolved above");
    let installed = registry
        .components
        .remove(component)
        .expect("component resolved above");
    let components_dir = registry.components_dir.clone();

    let file = project.resolve(&installed.file);
    match std::fs::remove_file(&file) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!("failed to remove {}: {error}", file.display()));
        }
    }

    if let Some(file_name) = installed.file.file_name().and_then(|name| name.to_str()) {
        module::undeclare(&project.resolve(&components_dir), file_name)?;
    }

    state.save(project)?;

    Ok(Removed {
        file: installed.file,
        registry: registry_name,
    })
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
            many.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ")
        )),
    }
}
