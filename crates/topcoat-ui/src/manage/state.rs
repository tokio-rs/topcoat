use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Registry, Source};

use super::project::Project;

/// The install-state file tracking which components a project has added.
pub(super) const STATE_FILE: &str = "components.toml";
/// The name of the built-in registry, used as the default when none is set.
pub(super) const DEFAULT_REGISTRY_NAME: &str = "topcoat";
/// The format version written into the install state.
const STATE_VERSION: u32 = 1;

/// The contents of `components.toml`. Components are grouped under the named
/// registry they were added from, so the same component name can be installed
/// from different registries and tracked independently.
#[derive(Serialize, Deserialize)]
pub(super) struct InstallState {
    #[serde(default = "default_state_version")]
    pub version: u32,
    /// The registry used by `add` when none is given. Optional: removing the
    /// default registry leaves the project with none, after which adding a
    /// component requires an explicit `--registry`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_registry: Option<String>,
    /// The base directory under which each registry's components are installed:
    /// a registry added later gets `<components_dir>/<registry-name>` as its
    /// output directory. Set once at `init` time.
    #[serde(default = "default_components_dir")]
    pub components_dir: PathBuf,
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryState>,
}

/// One registry's location, install target, and tracked components.
#[derive(Serialize, Deserialize)]
pub(super) struct RegistryState {
    /// The registry location: a path, a `file://` path, or an `http(s)://` URL.
    pub url: String,
    /// Where this registry's components are installed. Set to
    /// `<components-dir>/<registry-name>` when the registry is first added.
    pub components_dir: PathBuf,
    #[serde(default)]
    pub components: BTreeMap<String, InstalledComponent>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct InstalledComponent {
    pub hash: String,
    pub file: PathBuf,
}

impl Default for InstallState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            default_registry: None,
            components_dir: default_components_dir(),
            registries: BTreeMap::new(),
        }
    }
}

impl RegistryState {
    /// Creates a registry whose components install under `components_dir/<name>`.
    pub(super) fn new(name: &str, url: String, components_dir: &Path) -> Self {
        Self {
            url,
            components_dir: components_dir.join(name),
            components: BTreeMap::new(),
        }
    }
}

impl InstallState {
    pub(super) fn load(project: &Project) -> Result<Self, String> {
        let path = project.state_path();
        match std::fs::read_to_string(&path) {
            Ok(raw) => {
                let mut state: Self = toml::from_str(&raw)
                    .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
                if state.version > STATE_VERSION {
                    return Err(format!(
                        "{} has format version {} but this topcoat supports up to {}",
                        path.display(),
                        state.version,
                        STATE_VERSION
                    ));
                }
                // Normalize stored registry locations to a canonical relative
                // form so they compare and round-trip consistently.
                for registry in state.registries.values_mut() {
                    registry.url = project.to_stored(&registry.url);
                }
                Ok(state)
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Err(format!(
                "no install state at {}; run `topcoat ui init` first",
                path.display()
            )),
            Err(error) => Err(format!("failed to read {}: {error}", path.display())),
        }
    }

    pub(super) fn save(&self, project: &Project) -> Result<(), String> {
        let path = project.state_path();
        let body = toml::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize install state: {error}"))?;
        let contents = format!("# Topcoat UI install state. Managed by `topcoat ui`.\n{body}");
        std::fs::write(&path, contents)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    /// The built-in location for a registry name, if it has one. Only the
    /// built-in registry has a built-in location (the published registry).
    pub(super) fn default_url(name: &str) -> Option<String> {
        (name == DEFAULT_REGISTRY_NAME).then(|| crate::DEFAULT_REGISTRY.to_string())
    }

    /// Returns the named registry. A registry that does not exist is created
    /// only for the built-in `topcoat`, whose location is known; any other
    /// unknown registry errors, since registries are added explicitly with
    /// `topcoat ui registry add`.
    pub(super) fn registry_mut(&mut self, name: &str) -> Result<&mut RegistryState, String> {
        let components_dir = self.components_dir.clone();
        match self.registries.entry(name.to_string()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let url = Self::default_url(name).ok_or_else(|| {
                    format!("unknown registry `{name}`; add it with `topcoat ui registry add <url>`")
                })?;
                Ok(entry.insert(RegistryState::new(name, url, &components_dir)))
            }
        }
    }

    /// Resolves the install-state registry name for a cross-registry dependency
    /// at `url`, whose registry declares its own `name`. If a registry with the
    /// same URL is already tracked it is reused; otherwise the registry is added
    /// under its declared name, erroring if that name is already taken by a
    /// different location.
    pub(super) fn resolve_registry(&mut self, url: &str, name: &str) -> Result<String, String> {
        if let Some(existing) = self
            .registries
            .iter()
            .find(|(_, registry)| registry.url == url)
            .map(|(existing, _)| existing.clone())
        {
            return Ok(existing);
        }

        if self.registries.contains_key(name) {
            return Err(format!(
                "cannot add registry {url} as `{name}`: that name is already used for a different location"
            ));
        }

        let registry = RegistryState::new(name, url.to_string(), &self.components_dir);
        self.registries.insert(name.to_string(), registry);
        Ok(name.to_string())
    }

    /// Writes a fresh install state for a project that has none, seeding it with
    /// the initial default registry so the other commands always have a registry
    /// to work against without synthesizing one on the fly.
    ///
    /// `components_dir` overrides where components are installed (default
    /// `src/components`). With a `url`, that registry becomes the default and is
    /// loaded to learn its declared name; without one, the built-in `topcoat`
    /// registry is used. Errors if an install state already exists rather than
    /// clobbering it.
    pub(super) async fn create(
        project: &Project,
        components_dir: Option<PathBuf>,
        url: Option<String>,
    ) -> Result<Self, String> {
        let path = project.state_path();
        if path.exists() {
            return Err(format!(
                "{} already exists; the project is already initialized",
                path.display()
            ));
        }

        let mut state = Self::default();
        if let Some(components_dir) = components_dir {
            state.components_dir = components_dir;
        }

        // Resolve the initial default registry: with a url, load it to learn the
        // name it declares for itself; otherwise fall back to the built-in one.
        let (name, url) = match url {
            Some(url) => {
                let stored = project.to_stored(&url);
                let working = project.to_working(&stored);
                let registry = Registry::load(Source::parse(&working))
                    .await
                    .map_err(|error| format!("failed to load registry {working}: {error}"))?;
                (registry.name().to_string(), stored)
            }
            None => (
                DEFAULT_REGISTRY_NAME.to_string(),
                Self::default_url(DEFAULT_REGISTRY_NAME).expect("built-in registry has a location"),
            ),
        };

        let registry = RegistryState::new(&name, url, &state.components_dir);
        state.default_registry = Some(name.clone());
        state.registries.insert(name, registry);

        state.save(project)?;
        Ok(state)
    }
}

fn default_state_version() -> u32 {
    STATE_VERSION
}

/// The default base directory for component install output.
fn default_components_dir() -> PathBuf {
    PathBuf::from("src/components")
}
