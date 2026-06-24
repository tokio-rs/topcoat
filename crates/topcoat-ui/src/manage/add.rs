use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::{DEFAULT_REGISTRY_CRATE, Dependency, Registry, content_hash};

use super::Confirm;
use super::module;
use super::package::Package;
use super::state::{InstallState, InstalledComponent};
use super::workspace::Workspace;

/// What to add and from where.
pub struct AddOptions {
    /// Names of the components to add (e.g. `button`). Each is resolved and
    /// installed along with its transitive dependencies.
    pub components: Vec<String>,
    /// Registry crate to add from (defaults to the built-in default registry).
    pub registry: Option<String>,
    /// Overwrite the component file if it already exists.
    pub overwrite: bool,
}

/// A component written into the package by [`add`].
pub struct AddedComponent {
    /// The component's name.
    pub name: String,
    /// The package-relative path of the written file.
    pub file: PathBuf,
    /// The registry crate it was added from.
    pub registry: String,
}

/// The result of [`add`].
pub enum AddOutcome {
    /// Nothing was written; every needed file was already present.
    UpToDate,
    /// One or more components were written.
    Added(Vec<AddedComponent>),
}

/// A component still to be planned: which registry crate it lives in, its name,
/// and whether it is the component the user explicitly asked for (the root)
/// versus one pulled in as a dependency.
struct Pending {
    registry: String,
    component: String,
    root: bool,
}

/// A file to write once planning has fully succeeded.
struct PlannedWrite {
    name: String,
    dir: PathBuf,
    file: PathBuf,
    relative_file: PathBuf,
    file_name: String,
    contents: String,
    registry: String,
}

/// A previously installed component to remove (because it is being replaced by
/// a component from a different registry that occupies the same file).
struct PlannedRemoval {
    file: PathBuf,
    dir: PathBuf,
    file_name: String,
}

/// Adds a component (and its transitive dependencies) to the package.
///
/// The operation is transactional: it walks the requested component and its
/// dependencies, loading registries and reading sources without touching disk,
/// and only commits the writes once everything resolves. Interactive decisions
/// (pulling from a non-default registry, or replacing a file owned by another
/// registry) are delegated to `confirm`.
///
/// # Errors
///
/// Returns an error if the install state or workspace cannot be loaded, a
/// requested component or its registry cannot be resolved, a confirmation
/// prompt is declined, an existing file would be overwritten without
/// `overwrite`, or any file write, module declaration, or state save fails.
///
/// # Panics
///
/// Panics if a registry found to conflict with a new component is no longer
/// present in the install state when its old component is removed. This is an
/// internal invariant: the conflict was discovered by iterating the state, so
/// the registry must still be tracked.
pub fn add(
    package: &Package,
    options: &AddOptions,
    confirm: &mut Confirm<'_>,
) -> Result<AddOutcome, String> {
    let mut state = InstallState::load(package)?;
    let workspace = Workspace::load(package)?;

    // Phase 1: plan. Walk the requested components and their transitive
    // dependencies, loading registries and reading sources, but touch nothing on
    // disk. Any failure here (missing component, registry not a dependency)
    // leaves the package untouched.
    let mut registries: HashMap<String, Registry> = HashMap::new();
    let mut visited: HashSet<(String, String)> = HashSet::new();
    let mut queue: VecDeque<Pending> = VecDeque::new();

    // Choose the registry to add from for each requested component and seed it as
    // a root of the dependency walk. With --registry it is used directly;
    // otherwise the default registry is preferred, and pulling a component the
    // default registry does not offer requires confirming a non-default registry
    // (or passing --registry).
    for component in &options.components {
        let root_registry = resolve_root_registry(
            component,
            options.registry.as_deref(),
            &workspace,
            &mut registries,
            confirm,
        )?;
        queue.push_back(Pending {
            registry: root_registry,
            component: component.clone(),
            root: true,
        });
    }

    let mut writes: Vec<PlannedWrite> = Vec::new();
    let mut removals: Vec<PlannedRemoval> = Vec::new();

    while let Some(pending) = queue.pop_front() {
        if !visited.insert((pending.registry.clone(), pending.component.clone())) {
            continue;
        }

        // All registries install into one flat directory.
        let components_dir = state.components_dir.clone();

        let registry = load_registry(&mut registries, &workspace, &pending.registry)?;

        let component = registry.get(&pending.component).ok_or_else(|| {
            let available: Vec<&str> = registry.names().collect();
            format!(
                "unknown component `{}` in registry `{}`; available: {}",
                pending.component,
                pending.registry,
                available.join(", ")
            )
        })?;

        let relative_file = components_dir.join(component.file_name());
        let dir = package.resolve(&components_dir);
        let file = dir.join(component.file_name());

        // A file may hold only one component. If a different installed component
        // (from another registry) already occupies this file, offer to remove it
        // so this one can take its place. Same-named components that resolve to
        // different files do not collide and are left untouched.
        let mut replacing = false;
        if let Some((other_registry, other_component)) =
            find_file_conflict(&state, &pending.registry, component.name(), &relative_file)
        {
            let prompt = format!(
                "{} is already provided by `{other_component}` from `{other_registry}`. Replace it with `{}` from `{}`?",
                relative_file.display(),
                component.name(),
                pending.registry
            );
            if !confirm(&prompt)? {
                return Err(format!(
                    "aborted; {} is already provided by `{other_component}` from `{other_registry}`",
                    relative_file.display()
                ));
            }

            let registry = state
                .registries
                .get_mut(&other_registry)
                .expect("conflicting registry exists");
            if let Some(removed) = registry.components.remove(&other_component)
                && let Some(file_name) = removed.file.file_name().and_then(|name| name.to_str())
            {
                removals.push(PlannedRemoval {
                    file: package.resolve(&removed.file),
                    dir: package.resolve(&components_dir),
                    file_name: file_name.to_string(),
                });
            }
            replacing = true;
        }

        let exists = file.exists();
        if exists && pending.root && !options.overwrite && !replacing {
            return Err(format!(
                "{} already exists; pass --overwrite to replace it",
                relative_file.display()
            ));
        }

        // Read the source once: it is hashed to record the component's version in
        // the install state, and reused as the file contents when written.
        let contents = component
            .read_source()
            .map_err(|error| format!("failed to read component `{}`: {error}", component.name()))?;
        let hash = content_hash(&contents);

        // Write the source unless it is already present. Dependencies never
        // clobber existing files; only the root (or a replacement) rewrites.
        if !exists || (pending.root && options.overwrite) || replacing {
            writes.push(PlannedWrite {
                name: component.name().to_string(),
                dir: dir.clone(),
                file: file.clone(),
                relative_file: relative_file.clone(),
                file_name: component.file_name().to_string(),
                contents,
                registry: pending.registry.clone(),
            });
        }

        // Collect dependencies before recording the component, so the borrow on
        // the registry cache is released before the state is mutated.
        let dependencies = component.dependencies().to_vec();

        state.registry_mut(&pending.registry).components.insert(
            component.name().to_string(),
            InstalledComponent {
                hash,
                file: relative_file,
            },
        );

        for dependency in dependencies {
            // A dependency names a registry crate directly: `Same` for the
            // current one, `Other` for another (which must itself be a package
            // dependency, enforced when its registry is loaded on pop).
            let (registry, component) = match dependency {
                Dependency::Same(name) => (pending.registry.clone(), name),
                Dependency::Other { registry, name } => (registry, name),
            };
            queue.push_back(Pending {
                registry,
                component,
                root: false,
            });
        }
    }

    // Phase 2: commit. Everything resolved, so remove anything being replaced,
    // write the files, wire up the module declarations, and persist the state.
    // Reject an ambiguous module layout up front, before touching any file.
    module::check(&package.resolve(&state.components_dir))?;
    for removal in &removals {
        match std::fs::remove_file(&removal.file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "failed to remove {}: {error}",
                    removal.file.display()
                ));
            }
        }
        module::undeclare(&removal.dir, &removal.file_name)?;
    }
    for write in &writes {
        std::fs::create_dir_all(&write.dir)
            .map_err(|error| format!("failed to create {}: {error}", write.dir.display()))?;
        std::fs::write(&write.file, &write.contents)
            .map_err(|error| format!("failed to write {}: {error}", write.file.display()))?;
        module::declare(&write.dir, &write.file_name)?;
    }
    state.save(package)?;

    if writes.is_empty() {
        Ok(AddOutcome::UpToDate)
    } else {
        Ok(AddOutcome::Added(
            writes
                .into_iter()
                .map(|write| AddedComponent {
                    name: write.name,
                    file: write.relative_file,
                    registry: write.registry,
                })
                .collect(),
        ))
    }
}

/// Determines which registry the requested component should be added from.
///
/// With an explicit `--registry`, that registry crate is used and must offer the
/// component. Otherwise the default registry is preferred; if it does not offer
/// the component, the package's other dependency registries are searched and the
/// user is asked to confirm pulling from a non-default registry.
fn resolve_root_registry(
    component: &str,
    registry: Option<&str>,
    workspace: &Workspace,
    registries: &mut HashMap<String, Registry>,
    confirm: &mut Confirm<'_>,
) -> Result<String, String> {
    if let Some(name) = registry {
        let loaded = load_registry(registries, workspace, name)?;
        if loaded.get(component).is_none() {
            let available: Vec<&str> = loaded.names().collect();
            return Err(format!(
                "unknown component `{component}` in registry `{name}`; available: {}",
                available.join(", ")
            ));
        }
        return Ok(name.to_string());
    }

    // Prefer the default registry whenever it offers the component. It may not
    // offer it (or, for a project that does not depend on `topcoat`, not load at
    // all), in which case fall through to the package's other registries.
    let default = DEFAULT_REGISTRY_CRATE;
    let offers_default = match load_registry(registries, workspace, default) {
        Ok(registry) => registry.get(component).is_some(),
        Err(_) => false,
    };
    if offers_default {
        return Ok(default.to_string());
    }

    // Not in the default registry: look for it among the package's other
    // dependency registries, skipping any that fail to load.
    let others: Vec<String> = workspace
        .available_registries()
        .into_iter()
        .filter(|name| name != default)
        .collect();

    let mut offering: Vec<String> = Vec::new();
    for name in &others {
        let offers = match load_registry(registries, workspace, name) {
            Ok(registry) => registry.get(component).is_some(),
            Err(_) => false,
        };
        if offers {
            offering.push(name.clone());
        }
    }

    match offering.as_slice() {
        [] => Err(format!(
            "unknown component `{component}`: not in the default registry `{default}` or any dependency registry"
        )),
        [name] => {
            let prompt = format!(
                "`{component}` is not in the default registry `{default}`. Add it from `{name}` instead?"
            );
            if confirm(&prompt)? {
                Ok(name.clone())
            } else {
                Err(format!(
                    "aborted; pass `--registry {name}` to add `{component}` from it"
                ))
            }
        }
        many => Err(format!(
            "`{component}` is not in the default registry `{default}` but is available in {}; pass --registry to choose",
            many.join(", ")
        )),
    }
}

/// Finds an installed component, other than the one being installed, whose file
/// is the same package-relative path (a file collision). Same-named
/// components from different registries that map to different files do not
/// collide and are not reported.
fn find_file_conflict(
    state: &InstallState,
    registry: &str,
    component: &str,
    file: &Path,
) -> Option<(String, String)> {
    for (registry_name, registry_state) in &state.registries {
        for (component_name, installed) in &registry_state.components {
            if (registry_name.as_str(), component_name.as_str()) != (registry, component)
                && installed.file == file
            {
                return Some((registry_name.clone(), component_name.clone()));
            }
        }
    }
    None
}

/// Loads the registry crate `name`, caching it so each registry is resolved and
/// read once. Resolving validates that the crate is a usable registry dependency.
fn load_registry<'a>(
    cache: &'a mut HashMap<String, Registry>,
    workspace: &Workspace,
    name: &str,
) -> Result<&'a Registry, String> {
    if !cache.contains_key(name) {
        let dir = workspace.registry_dir(name)?;
        let loaded = Registry::load(dir)
            .map_err(|error| format!("failed to load registry `{name}`: {error}"))?;
        cache.insert(name.to_string(), loaded);
    }
    Ok(&cache[name])
}
