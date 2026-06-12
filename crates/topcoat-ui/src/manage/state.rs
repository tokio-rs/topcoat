use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_registry_name")]
    pub default_registry: String,
    /// The base directory under which each registry's components are installed:
    /// a registry added later gets `<base_dir>/<registry-name>` as its output
    /// directory. Set once at `init` time.
    #[serde(default = "default_base_dir")]
    pub base_dir: PathBuf,
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryState>,
}

/// One registry's location, install target, and tracked components.
#[derive(Serialize, Deserialize)]
pub(super) struct RegistryState {
    /// The registry location: a path, a `file://` path, or an `http(s)://` URL.
    pub url: String,
    /// Where this registry's components are installed. Set to
    /// `<base-dir>/<registry-name>` when the registry is first added.
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
            default_registry: default_registry_name(),
            base_dir: default_base_dir(),
            registries: BTreeMap::new(),
        }
    }
}

impl RegistryState {
    /// Creates a registry whose components install under `base_dir/<name>`.
    pub(super) fn new(name: &str, url: String, base_dir: &Path) -> Self {
        Self {
            url,
            components_dir: base_dir.join(name),
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

    /// Returns the named registry, creating it if necessary. A given `url` sets
    /// or overrides the registry's location. Creating a registry that does not
    /// exist requires a `url`, except the built-in registry.
    pub(super) fn registry_mut(
        &mut self,
        name: &str,
        url: Option<String>,
    ) -> Result<&mut RegistryState, String> {
        let base_dir = self.base_dir.clone();
        match self.registries.entry(name.to_string()) {
            Entry::Occupied(entry) => {
                let registry = entry.into_mut();
                if let Some(url) = url {
                    registry.url = url;
                }
                Ok(registry)
            }
            Entry::Vacant(entry) => {
                let url = url
                    .or_else(|| Self::default_url(name))
                    .ok_or_else(|| format!("unknown registry `{name}`; pass --url to define it"))?;
                Ok(entry.insert(RegistryState::new(name, url, &base_dir)))
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

        let registry = RegistryState::new(name, url.to_string(), &self.base_dir);
        self.registries.insert(name.to_string(), registry);
        Ok(name.to_string())
    }

    /// Writes a fresh install state for a project that has none, recording
    /// `base_dir` (default `src/components`) as the base for component output.
    /// Errors if an install state already exists rather than clobbering it.
    pub(super) fn create(project: &Project, base_dir: Option<PathBuf>) -> Result<Self, String> {
        let path = project.state_path();
        if path.exists() {
            return Err(format!(
                "{} already exists; the project is already initialized",
                path.display()
            ));
        }
        let mut state = Self::default();
        if let Some(base_dir) = base_dir {
            state.base_dir = base_dir;
        }
        state.save(project)?;
        Ok(state)
    }
}

fn default_state_version() -> u32 {
    STATE_VERSION
}

fn default_registry_name() -> String {
    DEFAULT_REGISTRY_NAME.to_string()
}

/// The default base directory for component install output.
fn default_base_dir() -> PathBuf {
    PathBuf::from("src/components")
}
