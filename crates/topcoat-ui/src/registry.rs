use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The manifest file naming the components within a registry.
pub const MANIFEST_FILE: &str = "registry.toml";

/// The `registry.toml` format version this build understands. Stored in the
/// manifest's `version` field so older and newer formats can be told apart; a
/// manifest declaring a newer version than this is rejected.
pub const MANIFEST_VERSION: u32 = 1;

/// The crate name of the registry used when a project does not specify one: the
/// `topcoat` facade crate, which carries the default component registry and is
/// always a direct dependency of a Topcoat project.
pub const DEFAULT_REGISTRY_CRATE: &str = "topcoat";

/// The parsed `registry.toml` manifest. Written by hand: it records no hashes,
/// since a component's hash is computed from its source (see [`content_hash`]).
/// The registry's identity is its crate name, so the manifest names only the
/// format version and the components.
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
    /// Loads a registry by reading and parsing the `registry.toml` in `dir` (a
    /// registry crate's declared registry directory).
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

    /// Computes the component's content hash by reading and hashing its source
    /// (see [`content_hash`]).
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

/// A single theme within a [`Registry`]: a CSS file that becomes a project's
/// Tailwind input, copied into the project at `init` time.
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

    /// The file name written into the user's project. Every theme installs to
    /// the same `styles.css` (it becomes the project's Tailwind input), rather
    /// than carrying its registry source name (e.g. `neutral.css`) into the project.
    #[must_use]
    pub fn file_name(&self) -> &'static str {
        "styles.css"
    }

    /// Computes the theme's content hash by reading and hashing its source (see
    /// [`content_hash`]).
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

/// Computes the content hash recorded for a component, the sha256 of its source
/// prefixed with `sha256:`. Hashing the same source always yields the same
/// value, so a project can tell its installed component apart from an updated
/// one by comparing the hash it recorded against a fresh hash of the registry's
/// current source.
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
