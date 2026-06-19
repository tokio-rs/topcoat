use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::package::Package;

/// The install-state file tracking which components a package has added.
pub(super) const STATE_FILE: &str = "components.toml";
/// The format version written into the install state.
const STATE_VERSION: u32 = 1;

/// The contents of `components.toml`. Components are grouped under the registry
/// crate they were added from, so the same component name can be installed from
/// different registries and tracked independently.
#[derive(Serialize, Deserialize)]
pub(super) struct InstallState {
    #[serde(default = "default_state_version")]
    pub version: u32,
    /// The single directory all registries' components are installed into, flat.
    /// Set once at `init` time.
    #[serde(default = "default_components_dir")]
    pub components_dir: PathBuf,
    /// The theme installed at `init` time, if one was chosen. The CSS itself
    /// lives in the package; this records which theme it came from so updates
    /// can be surfaced the same way components are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<InstalledTheme>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub registries: BTreeMap<String, RegistryState>,
}

/// The theme a package installed: which registry and theme it came from, the
/// hash of the theme's source when installed, and where the CSS was written.
#[derive(Serialize, Deserialize)]
pub(super) struct InstalledTheme {
    pub name: String,
    pub registry: String,
    pub hash: String,
    pub file: PathBuf,
}

/// One registry crate's tracked components.
#[derive(Default, Serialize, Deserialize)]
pub(super) struct RegistryState {
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
            components_dir: default_components_dir(),
            theme: None,
            registries: BTreeMap::new(),
        }
    }
}

impl InstallState {
    pub(super) fn load(package: &Package) -> Result<Self, String> {
        let path = package.state_path();
        match std::fs::read_to_string(&path) {
            Ok(raw) => {
                let state: Self = toml::from_str(&raw)
                    .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
                if state.version > STATE_VERSION {
                    return Err(format!(
                        "{} has format version {} but this topcoat supports up to {}",
                        path.display(),
                        state.version,
                        STATE_VERSION
                    ));
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

    pub(super) fn save(&self, package: &Package) -> Result<(), String> {
        let path = package.state_path();
        let body = toml::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize install state: {error}"))?;
        let contents = format!("# Topcoat UI install state. Managed by `topcoat ui`.\n{body}");
        std::fs::write(&path, contents)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    /// Returns the registry's tracked state, creating an empty entry the first
    /// time a component is added from it. The crate has already been validated as
    /// a usable registry by the caller, so this only manages bookkeeping.
    pub(super) fn registry_mut(&mut self, name: &str) -> &mut RegistryState {
        self.registries.entry(name.to_string()).or_default()
    }

    /// Writes a fresh install state for a package that has none, recording where
    /// components install. Registries are not tracked here; they are discovered
    /// from the package's dependencies, so this only sets up the file the other
    /// commands require. Errors if an install state already exists rather than
    /// clobbering it.
    pub(super) fn create(
        package: &Package,
        components_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let path = package.state_path();
        if path.exists() {
            return Err(format!(
                "{} already exists; the package is already initialized",
                path.display()
            ));
        }

        let mut state = Self::default();
        if let Some(components_dir) = components_dir {
            state.components_dir = components_dir;
        }

        state.save(package)?;
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
