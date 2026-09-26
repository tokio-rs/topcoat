use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};

use super::{
    Confirm, module,
    package::Package,
    state::{InstallState, InstalledComponent},
    workspace::Workspace,
};
use crate::{DEFAULT_REGISTRY, Dependency, Registry, content_hash};

/// Options for installing components.
pub struct AddOptions {
    /// Which components to add. Each is resolved and installed along with its
    /// transitive dependencies.
    pub selection: Selection,
    /// Registry crate to add from (defaults to the built-in default registry).
    pub registry: Option<String>,
    /// Overwrite the component file if it already exists.
    pub overwrite: bool,
}

/// The components an [`add`] call installs.
pub enum Selection {
    /// Components named individually (e.g. `button`). A named component whose
    /// file already exists is an error unless overwriting.
    Named(Vec<String>),
    /// Every component the registry offers. Components whose files already
    /// exist are skipped unless overwriting.
    All,
}

/// What [`add`] did with one component.
pub struct AddEntry {
    /// The component's name.
    pub name: String,
    /// The package-relative path of the component's file.
    pub file: PathBuf,
    /// The registry crate it was added from.
    pub registry: String,
    /// What happened to the file.
    pub action: AddAction,
}

/// The action [`add`] took for a component.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddAction {
    /// The file was written for the first time.
    Installed,
    /// An existing file was replaced with the registry's source.
    Overwritten,
    /// The file already existed and was left untouched.
    Skipped,
}

/// A requested component or dependency awaiting installation planning.
struct Pending {
    registry: String,
    component: String,
    origin: Origin,
}

/// How a pending component entered the plan, which decides what happens when
/// its file already exists.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// Named by the caller: an existing file is an error unless overwriting.
    Named,
    /// Swept in by selecting every registry component: an existing file is
    /// skipped unless overwriting.
    Swept,
    /// Pulled in as a dependency: an existing file is never touched.
    Dependency,
}

impl Origin {
    /// Whether an existing file is rewritten when overwriting is requested.
    fn overwritable(self) -> bool {
        matches!(self, Self::Named | Self::Swept)
    }
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
    /// Whether the file exists and is being replaced.
    existed: bool,
}

/// An installed component to replace because another registry supplies the same
/// destination file.
struct PlannedRemoval {
    file: PathBuf,
    dir: PathBuf,
    file_name: String,
}

/// Installs the requested components and their dependencies.
///
/// Resolves registries, reads sources, and obtains confirmations before writing files.
/// The `confirm` callback decides whether to use another registry or replace a file
/// owned by one. File writes begin only after planning succeeds, but a later I/O
/// failure can leave a partial installation.
///
/// Returns an entry for every file written and for every selected component that
/// was skipped because its file already exists. Dependencies whose files already
/// exist are left untouched and not reported.
///
/// # Errors
///
/// Returns an error if the install state or workspace cannot be loaded, a
/// requested component or its registry cannot be resolved, a confirmation
/// prompt is declined, a named component's file already exists without
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
) -> Result<Vec<AddEntry>, String> {
    let mut state = InstallState::load(package)?;
    let workspace = Workspace::load(package)?;

    // Phase 1: plan. Walk the requested components and their transitive
    // dependencies, loading registries and reading sources, but touch nothing on
    // disk. Any failure here (missing component, registry not a dependency)
    // leaves the package untouched.
    let mut registries: HashMap<String, Registry> = HashMap::new();
    let mut visited: HashSet<(String, String)> = HashSet::new();
    let mut queue: VecDeque<Pending> = VecDeque::new();

    // Seed the roots of the dependency walk.
    match &options.selection {
        // Choose the registry to add from for each named component. With
        // --registry it is used directly; otherwise the default registry is
        // preferred, and pulling a component the default registry does not offer
        // requires confirming a non-default registry (or passing --registry).
        Selection::Named(components) => {
            for component in components {
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
                    origin: Origin::Named,
                });
            }
        }
        // Every component of one registry: the one given with --registry, or the
        // default registry. Nothing needs confirming since no other registry is
        // consulted.
        Selection::All => {
            let name = options.registry.as_deref().unwrap_or(DEFAULT_REGISTRY);
            let registry = load_registry(&mut registries, &workspace, name)?;
            queue.extend(registry.names().map(|component| Pending {
                registry: name.to_string(),
                component: component.to_string(),
                origin: Origin::Swept,
            }));
        }
    }

    let mut writes: Vec<PlannedWrite> = Vec::new();
    let mut removals: Vec<PlannedRemoval> = Vec::new();
    let mut skipped: Vec<AddEntry> = Vec::new();

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
        if exists && !options.overwrite && !replacing {
            match pending.origin {
                Origin::Named => {
                    return Err(format!(
                        "{} already exists; pass --overwrite to replace it",
                        relative_file.display()
                    ));
                }
                Origin::Swept => skipped.push(AddEntry {
                    name: component.name().to_string(),
                    file: relative_file.clone(),
                    registry: pending.registry.clone(),
                    action: AddAction::Skipped,
                }),
                Origin::Dependency => {}
            }
        }

        // Read the source once: it is hashed to record the component's version in
        // the install state, and reused as the file contents when written.
        let contents = component
            .read_source()
            .map_err(|error| format!("failed to read component `{}`: {error}", component.name()))?;
        let hash = content_hash(&contents);

        // Write the source unless it is already present. Dependencies never
        // clobber existing files; only a root being overwritten (or a
        // replacement) rewrites.
        if !exists || (pending.origin.overwritable() && options.overwrite) || replacing {
            writes.push(PlannedWrite {
                name: component.name().to_string(),
                dir: dir.clone(),
                file: file.clone(),
                relative_file: relative_file.clone(),
                file_name: component.file_name().to_string(),
                contents,
                registry: pending.registry.clone(),
                existed: exists,
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
                origin: Origin::Dependency,
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

    let mut entries: Vec<AddEntry> = writes
        .into_iter()
        .map(|write| AddEntry {
            name: write.name,
            file: write.relative_file,
            registry: write.registry,
            action: if write.existed {
                AddAction::Overwritten
            } else {
                AddAction::Installed
            },
        })
        .collect();
    entries.append(&mut skipped);
    Ok(entries)
}

/// Selects a registry for a component.
///
/// An explicit registry must provide the component. Otherwise, prefer the default
/// registry. Ask for confirmation before using a component found in another dependency
/// registry.
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
    let default = DEFAULT_REGISTRY;
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

/// Finds another installed component using the same destination path. Components with
/// the same name but different paths do not conflict.
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

/// Resolves and caches a registry, validating that it is an allowed dependency.
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
