use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use clap::Args;
use console::style;
use topcoat_ui::{Dependency, Registry, Source};

use super::module;
use super::state::{InstallState, InstalledComponent, STATE_FILE};

#[derive(Args)]
pub(super) struct AddCommand {
    /// Name of the component to add (e.g. `button`)
    component: String,
    /// Named registry to add from (defaults to the project's default registry)
    #[arg(short, long)]
    registry: Option<String>,
    /// Registry location (a path, `file://` path, or `http(s)://` URL); sets or
    /// overrides the location stored for the registry
    #[arg(short, long)]
    url: Option<String>,
    /// Overwrite the component file if it already exists
    #[arg(short, long)]
    force: bool,
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
    components_dir: PathBuf,
    file: PathBuf,
    file_name: String,
    contents: String,
    registry: String,
}

impl AddCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let state_path = Path::new(STATE_FILE);
        let mut state = InstallState::load(state_path)?;

        // Resolve (and, with --url, create or update) the named registry. The
        // location is sticky: it stays recorded for later commands.
        let root_registry = self
            .registry
            .clone()
            .unwrap_or_else(|| state.default_registry.clone());
        state.registry_mut(&root_registry, self.url)?;

        // Phase 1 — plan. Walk the requested component and its transitive
        // dependencies, loading registries and fetching sources, but touch
        // nothing on disk. Any failure here (missing component, unreachable
        // registry, registry-name conflict) leaves the project untouched.
        let mut registries: HashMap<String, Registry> = HashMap::new();
        let mut visited: HashSet<(String, String)> = HashSet::new();
        let mut queue: VecDeque<Pending> = VecDeque::new();
        queue.push_back(Pending {
            registry: root_registry,
            component: self.component.clone(),
            root: true,
        });

        let mut writes: Vec<PlannedWrite> = Vec::new();

        while let Some(pending) = queue.pop_front() {
            if !visited.insert((pending.registry.clone(), pending.component.clone())) {
                continue;
            }

            let (url, components_dir) = {
                let registry = state
                    .registries
                    .get(&pending.registry)
                    .expect("registry resolved before queueing");
                (registry.url.clone(), registry.components_dir.clone())
            };

            let registry = load_registry(&mut registries, &url).await?;

            let component = registry.get(&pending.component).ok_or_else(|| {
                let available: Vec<&str> = registry.names().collect();
                format!(
                    "unknown component `{}` in registry `{}`; available: {}",
                    pending.component,
                    pending.registry,
                    available.join(", ")
                )
            })?;

            let file = components_dir.join(component.file_name());
            let exists = file.exists();
            if exists && pending.root && !self.force {
                return Err(format!(
                    "{} already exists; pass --force to overwrite",
                    file.display()
                ));
            }

            // Write the source unless it is already present — dependencies never
            // clobber existing files; only the root respects --force.
            if !exists || (pending.root && self.force) {
                let contents = component
                    .fetch_source()
                    .await
                    .map_err(|error| format!("failed to read component `{}`: {error}", component.name()))?;
                writes.push(PlannedWrite {
                    components_dir: components_dir.clone(),
                    file: file.clone(),
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
                        file: file.clone(),
                    },
                );

            // Collect dependencies before releasing the borrow on the registry
            // cache, so the loop below can load further registries into it.
            let dependencies = component.dependencies().to_vec();
            for dependency in dependencies {
                let (registry, component) = match dependency {
                    Dependency::Same(name) => (pending.registry.clone(), name),
                    Dependency::Other { registry: location, name } => {
                        // Resolve the dependency's registry relative to the
                        // registry that declared it, so `file://../other` points
                        // at a sibling of the current registry.
                        let resolved = Source::parse(&url).resolve(&location);
                        let declared = load_registry(&mut registries, &resolved).await?.name();
                        (state.resolve_registry(&resolved, declared)?, name)
                    }
                };
                queue.push_back(Pending {
                    registry,
                    component,
                    root: false,
                });
            }
        }

        // Phase 2 — commit. Everything resolved, so write the files, wire up the
        // module declarations, and persist the install state.
        for write in &writes {
            std::fs::create_dir_all(&write.components_dir)
                .map_err(|error| format!("failed to create {}: {error}", write.components_dir.display()))?;
            std::fs::write(&write.file, &write.contents)
                .map_err(|error| format!("failed to write {}: {error}", write.file.display()))?;
            module::declare(&write.components_dir, &write.file_name)?;
        }
        state.save(state_path)?;

        if writes.is_empty() {
            println!("{} already up to date", style("✓").green());
        } else {
            for write in &writes {
                println!(
                    "{} added {} from {}",
                    style("✓").green(),
                    style(write.file.display()).bold(),
                    write.registry
                );
            }
        }
        Ok(())
    }
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
