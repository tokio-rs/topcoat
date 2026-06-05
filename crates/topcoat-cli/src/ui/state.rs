use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The install-state file tracking which components a project has added.
pub(super) const STATE_FILE: &str = "components.toml";
/// The name of the built-in registry, used when none is specified.
pub(super) const DEFAULT_REGISTRY_NAME: &str = "default";
const DEFAULT_COMPONENTS_DIR: &str = "src/components/ui";

/// The contents of `components.toml`. Components are grouped under the named
/// registry they were added from, so the same component name can be installed
/// from different registries and tracked independently.
#[derive(Default, Serialize, Deserialize)]
pub(super) struct InstallState {
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryState>,
}

/// One registry's location, install target, and tracked components.
#[derive(Serialize, Deserialize)]
pub(super) struct RegistryState {
    /// The registry location: a path, a `file://` path, or an `http(s)://` URL.
    pub url: String,
    #[serde(default = "default_components_dir")]
    pub components_dir: PathBuf,
    #[serde(default)]
    pub components: BTreeMap<String, InstalledComponent>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct InstalledComponent {
    pub version: String,
    pub file: PathBuf,
}

impl RegistryState {
    pub(super) fn new(url: String) -> Self {
        Self {
            url,
            components_dir: default_components_dir(),
            components: BTreeMap::new(),
        }
    }
}

impl InstallState {
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                toml::from_str(&raw).map_err(|error| format!("failed to parse {}: {error}", path.display()))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("failed to read {}: {error}", path.display())),
        }
    }

    pub(super) fn save(&self, path: &Path) -> Result<(), String> {
        let body =
            toml::to_string_pretty(self).map_err(|error| format!("failed to serialize install state: {error}"))?;
        let contents = format!("# Topcoat UI install state. Managed by `topcoat ui add`.\n{body}");
        std::fs::write(path, contents).map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    /// The built-in location for a registry name, if it has one. Only the
    /// `default` registry has a built-in location (the published registry).
    pub(super) fn default_url(name: &str) -> Option<String> {
        (name == DEFAULT_REGISTRY_NAME).then(|| topcoat_ui::DEFAULT_REGISTRY.to_string())
    }

    /// Returns the named registry, creating it if necessary. A given `url` sets
    /// or overrides the registry's location. Creating a registry that does not
    /// exist requires a `url`, except the built-in `default` registry.
    pub(super) fn registry_mut(
        &mut self,
        name: &str,
        url: Option<String>,
    ) -> Result<&mut RegistryState, String> {
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
                Ok(entry.insert(RegistryState::new(url)))
            }
        }
    }
}

fn default_components_dir() -> PathBuf {
    PathBuf::from(DEFAULT_COMPONENTS_DIR)
}
