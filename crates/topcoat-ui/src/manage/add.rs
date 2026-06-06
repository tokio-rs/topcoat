use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::{Dependency, Registry, Source};

use super::Confirm;
use super::module;
use super::project::Project;
use super::state::{InstallState, InstalledComponent};

/// What to add and from where.
pub struct AddOptions {
    /// Names of the components to add (e.g. `button`). Each is resolved and
    /// installed along with its transitive dependencies.
    pub components: Vec<String>,
    /// Named registry to add from (defaults to the project's default registry).
    pub registry: Option<String>,
    /// Registry location (a path, `file://` path, or `http(s)://` URL); sets or
    /// overrides the location stored for the registry.
    pub url: Option<String>,
    /// Overwrite the component file if it already exists.
    pub force: bool,
}

/// A component written into the project by [`add`].
pub struct AddedComponent {
    /// The component's name.
    pub name: String,
    /// The project-relative path of the written file.
    pub file: PathBuf,
    /// The registry it was added from.
    pub registry: String,
}

/// The result of [`add`].
pub enum AddOutcome {
    /// Nothing was written — every needed file was already present.
    UpToDate,
    /// One or more components were written.
    Added(Vec<AddedComponent>),
}

/// A component still to be planned: which registry it lives in, its name, and
/// whether it is the component the user explicitly asked for (the root) versus
/// one pulled in as a dependency.
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

/// Adds a component (and its transitive dependencies) to the project.
///
/// The operation is transactional: it walks the requested component and its
/// dependencies, loading registries and fetching sources without touching disk,
/// and only commits the writes once everything resolves. Interactive decisions
/// — pulling from a non-default registry, or replacing a file owned by another
/// registry — are delegated to `confirm`.
pub async fn add(
    project: &Project,
    options: &AddOptions,
    confirm: &mut Confirm<'_>,
) -> Result<AddOutcome, String> {
    let mut state = InstallState::load(project)?;

    // Phase 1 — plan. Walk the requested components and their transitive
    // dependencies, loading registries and fetching sources, but touch nothing
    // on disk. Any failure here (missing component, unreachable registry,
    // registry-name conflict) leaves the project untouched.
    let mut registries: HashMap<String, Registry> = HashMap::new();
    let mut visited: HashSet<(String, String)> = HashSet::new();
    let mut queue: VecDeque<Pending> = VecDeque::new();

    // Choose the registry to add from for each requested component and seed it
    // as a root of the dependency walk. With --registry it is used directly;
    // otherwise the default registry is preferred, and pulling a component the
    // default registry does not offer requires confirming a non-default
    // registry (or passing --registry).
    for component in &options.components {
        let root_registry = resolve_root_registry(
            component,
            options.registry.as_deref(),
            options.url.as_deref(),
            project,
            &mut state,
            &mut registries,
            confirm,
        )
        .await?;
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

        let (stored_url, components_dir) = {
            let registry = state
                .registries
                .get(&pending.registry)
                .expect("registry resolved before queueing");
            (registry.url.clone(), registry.components_dir.clone())
        };
        let working_url = project.to_working(&stored_url);

        let registry = load_registry(&mut registries, &working_url).await?;

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
        let dir = project.resolve(&components_dir);
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
            if let Some(removed) = registry.components.remove(&other_component) {
                let removed_dir = registry.components_dir.clone();
                if let Some(file_name) = removed.file.file_name().and_then(|name| name.to_str()) {
                    removals.push(PlannedRemoval {
                        file: project.resolve(&removed.file),
                        dir: project.resolve(&removed_dir),
                        file_name: file_name.to_string(),
                    });
                }
            }
            replacing = true;
        }

        let exists = file.exists();
        if exists && pending.root && !options.force && !replacing {
            return Err(format!(
                "{} already exists; pass --force to overwrite",
                relative_file.display()
            ));
        }

        // Write the source unless it is already present — dependencies never
        // clobber existing files; only the root (or a replacement) rewrites.
        if !exists || (pending.root && options.force) || replacing {
            let contents = component
                .fetch_source()
                .await
                .map_err(|error| format!("failed to read component `{}`: {error}", component.name()))?;
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

        state
            .registries
            .get_mut(&pending.registry)
            .expect("registry resolved above")
            .components
            .insert(
                component.name().to_string(),
                InstalledComponent {
                    version: component.version().to_string(),
                    file: relative_file,
                },
            );

        // Collect dependencies before releasing the borrow on the registry
        // cache, so the loop below can load further registries into it.
        let dependencies = component.dependencies().to_vec();
        for dependency in dependencies {
            let (registry, component) = match dependency {
                Dependency::Same(name) => (pending.registry.clone(), name),
                Dependency::Other { registry: location, name } => {
                    // Resolve the dependency's registry relative to the registry
                    // that declared it, then store it relative to the project.
                    let resolved = Source::parse(&working_url)
                        .resolve(&location)
                        .map_err(|error| format!("failed to resolve dependency registry {location}: {error}"))?;
                    let stored = project.to_stored(&resolved);
                    let declared = load_registry(&mut registries, &project.to_working(&stored))
                        .await?
                        .name();
                    (state.resolve_registry(&stored, declared)?, name)
                }
            };
            queue.push_back(Pending {
                registry,
                component,
                root: false,
            });
        }
    }

    // Phase 2 — commit. Everything resolved, so remove anything being replaced,
    // write the files, wire up the module declarations, and persist the state.
    for removal in &removals {
        match std::fs::remove_file(&removal.file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("failed to remove {}: {error}", removal.file.display()));
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
    state.save(project)?;

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
/// With an explicit `--registry`, that registry is used and must offer the
/// component. Otherwise the default registry is preferred; if it does not offer
/// the component, the other registries are searched and the user is asked to
/// confirm pulling from a non-default registry.
async fn resolve_root_registry(
    component: &str,
    registry: Option<&str>,
    url: Option<&str>,
    project: &Project,
    state: &mut InstallState,
    registries: &mut HashMap<String, Registry>,
    confirm: &mut Confirm<'_>,
) -> Result<String, String> {
    if let Some(name) = registry {
        let stored = url.map(|url| project.to_stored(url));
        let working = {
            let registry = state.registry_mut(name, stored)?;
            project.to_working(&registry.url)
        };
        let loaded = load_registry(registries, &working).await?;
        if loaded.get(component).is_none() {
            let available: Vec<&str> = loaded.names().collect();
            return Err(format!(
                "unknown component `{component}` in registry `{name}`; available: {}",
                available.join(", ")
            ));
        }
        return Ok(name.to_string());
    }

    // A bare --url (no --registry) adds from the registry at that location,
    // named by the registry's own declared name rather than the default name.
    if let Some(url) = url {
        let stored = project.to_stored(url);
        let loaded = load_registry(registries, &project.to_working(&stored)).await?;
        let name = loaded.name().to_string();
        if loaded.get(component).is_none() {
            let available: Vec<&str> = loaded.names().collect();
            return Err(format!(
                "unknown component `{component}` in registry `{name}`; available: {}",
                available.join(", ")
            ));
        }
        // The first registry added to a fresh project becomes its default.
        let fresh = state.registries.is_empty();
        let resolved = state.resolve_registry(&stored, &name)?;
        if fresh {
            state.default_registry = resolved.clone();
        }
        return Ok(resolved);
    }

    // Prefer the default registry whenever it offers the component.
    let default = state.default_registry.clone();
    let default_url = {
        let registry = state.registry_mut(&default, None)?;
        project.to_working(&registry.url)
    };
    if load_registry(registries, &default_url)
        .await?
        .get(component)
        .is_some()
    {
        return Ok(default);
    }

    // Not in the default registry: look for it among the other registries.
    let others: Vec<(String, String)> = state
        .registries
        .iter()
        .filter(|(name, _)| **name != default)
        .map(|(name, registry)| (name.clone(), registry.url.clone()))
        .collect();

    let mut offering: Vec<String> = Vec::new();
    for (name, stored_url) in &others {
        if load_registry(registries, &project.to_working(stored_url))
            .await?
            .get(component)
            .is_some()
        {
            offering.push(name.clone());
        }
    }

    match offering.as_slice() {
        [] => Err(format!(
            "unknown component `{component}`: not in the default registry `{default}` or any other registry"
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
/// is the same project-relative path — i.e. a file collision. Same-named
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

/// Loads the registry at `url`, caching it so each registry is fetched once.
async fn load_registry<'a>(
    cache: &'a mut HashMap<String, Registry>,
    url: &str,
) -> Result<&'a Registry, String> {
    if !cache.contains_key(url) {
        let loaded = Registry::load(Source::parse(url))
            .await
            .map_err(|error| format!("failed to load registry {url}: {error}"))?;
        cache.insert(url.to_string(), loaded);
    }
    Ok(&cache[url])
}
