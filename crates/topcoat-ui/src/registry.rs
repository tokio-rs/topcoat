use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The manifest file naming the components within a registry.
pub const MANIFEST_FILE: &str = "registry.toml";

/// The newest supported registry manifest version. Manifests declaring a newer version
/// are rejected.
pub const MANIFEST_VERSION: u32 = 1;

/// The command-line and install-state name of the built-in registry.
pub const DEFAULT_REGISTRY: &str = "topcoat";

/// The crate providing the built-in registry. It is available through Topcoat's `ui`
/// feature without a separate direct dependency.
pub const DEFAULT_REGISTRY_CRATE: &str = "topcoat-ui-registry";

/// A registry manifest describing available themes and component sources.
#[derive(Deserialize)]
struct Manifest {
    /// The manifest format version (see [`MANIFEST_VERSION`]).
    version: u32,
    #[serde(default)]
    themes: BTreeMap<String, ThemeEntry>,
    #[serde(default)]
    components: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    source: String,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct ThemeEntry {
    source: String,
}

/// Another component that must be installed alongside a component.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// A component in the same registry, named directly.
    Same(String),
    /// A component in another registry, identified by that registry's crate
    /// name. The crate must itself be a dependency of the project.
    Other { registry: String, name: String },
}

/// A component registry loaded from a crate's registry directory.
pub struct Registry {
    dir: PathBuf,
    themes: BTreeMap<String, ThemeEntry>,
    components: BTreeMap<String, Entry>,
}

impl Registry {
    /// Loads the `registry.toml` manifest in `dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be read or parsed, or if it
    /// declares a format version newer than [`MANIFEST_VERSION`].
    pub fn load(dir: PathBuf) -> Result<Self, Error> {
        let manifest_path = dir.join(MANIFEST_FILE);
        let raw = std::fs::read_to_string(&manifest_path).map_err(|source| Error::Read {
            path: manifest_path,
            source,
        })?;
        let manifest: Manifest = toml::from_str(&raw)?;
        if manifest.version > MANIFEST_VERSION {
            return Err(Error::UnsupportedVersion {
                found: manifest.version,
                supported: MANIFEST_VERSION,
            });
        }
        Ok(Self {
            dir,
            themes: manifest.themes,
            components: manifest.components,
        })
    }

    /// The names of every component in the registry, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }

    /// Looks up a component by its registry name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Component<'_>> {
        self.components
            .get_key_value(name)
            .map(|(name, entry)| Component {
                name,
                entry,
                dir: &self.dir,
            })
    }

    /// The names of every theme the registry offers, sorted.
    pub fn theme_names(&self) -> impl Iterator<Item = &str> {
        self.themes.keys().map(String::as_str)
    }

    /// Looks up a theme by its registry name.
    #[must_use]
    pub fn theme(&self, name: &str) -> Option<Theme<'_>> {
        self.themes.get_key_value(name).map(|(name, entry)| Theme {
            name,
            entry,
            dir: &self.dir,
        })
    }
}

/// A single component within a [`Registry`].
pub struct Component<'a> {
    name: &'a str,
    entry: &'a Entry,
    dir: &'a Path,
}

impl Component<'_> {
    /// The name used to add the component, e.g. `button`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the content hash of the component's source.
    ///
    /// # Errors
    ///
    /// Returns an error if the component's source file cannot be read.
    pub fn hash(&self) -> Result<String, Error> {
        Ok(content_hash(&self.read_source()?))
    }

    /// The file name written into the user's components directory.
    #[must_use]
    pub fn file_name(&self) -> &str {
        Path::new(&self.entry.source)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.entry.source)
    }

    /// Reads the component's Rust source from the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if the source file cannot be read.
    pub fn read_source(&self) -> Result<String, Error> {
        let path = self.dir.join(&self.entry.source);
        std::fs::read_to_string(&path).map_err(|source| Error::Read { path, source })
    }

    /// The other components this component depends on.
    #[must_use]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.entry.dependencies
    }
}

/// A registry theme whose CSS can be installed as a project's Tailwind input.
pub struct Theme<'a> {
    name: &'a str,
    entry: &'a ThemeEntry,
    dir: &'a Path,
}

impl Theme<'_> {
    /// The name used to select the theme, e.g. `neutral`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
    }

    /// The destination file name for an installed theme, `styles.css`.
    #[must_use]
    pub fn file_name(&self) -> &'static str {
        "styles.css"
    }

    /// Returns the content hash of the theme's source.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme's source file cannot be read.
    pub fn hash(&self) -> Result<String, Error> {
        Ok(content_hash(&self.read_source()?))
    }

    /// Reads the theme's CSS source from the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if the source file cannot be read.
    pub fn read_source(&self) -> Result<String, Error> {
        let path = self.dir.join(&self.entry.source);
        std::fs::read_to_string(&path).map_err(|source| Error::Read { path, source })
    }
}

/// Returns the SHA-256 hash of `source`, prefixed with `sha256:`. Identical source
/// produces the same hash.
#[must_use]
pub fn content_hash(source: &str) -> String {
    format!("sha256:{}", hex(Sha256::digest(source.as_bytes()).as_ref()))
}

/// Lowercase hex encoding of a byte slice.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

/// An error loading a registry or one of its components.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path:?}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse registry manifest")]
    Parse(#[from] toml::de::Error),
    #[error(
        "registry manifest has format version {found}, but this build supports up to {supported}"
    )]
    UnsupportedVersion { found: u32, supported: u32 },
}
