use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The filename of the manifest in a registry directory.
pub const MANIFEST_FILE: &str = "registry.toml";

/// The newest `registry.toml` format version this crate can read.
///
/// [`Registry::load`] rejects a manifest whose `version` is higher.
pub const MANIFEST_VERSION: u32 = 1;

/// The name of the built-in registry.
///
/// Commands use this registry when no other is named. It is the name used on
/// the `topcoat ui` command line and in a project's install state. It refers
/// to the [`DEFAULT_REGISTRY_CRATE`] crate.
pub const DEFAULT_REGISTRY: &str = "topcoat";

/// The crate that provides the built-in registry, which is named
/// [`DEFAULT_REGISTRY`].
///
/// Other registries must be direct dependencies of the project. This one does
/// not have to be, because the `ui` feature of `topcoat` depends on it.
pub const DEFAULT_REGISTRY_CRATE: &str = "topcoat-ui-registry";

/// The parsed `registry.toml` manifest. It records no hashes, because the hash
/// of a component is computed from its source (see [`content_hash`]). The
/// registry is identified by its crate name, so the manifest only holds the
/// format version, the themes, and the components.
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

/// A component that must be installed together with another component.
///
/// In `registry.toml`, a dependency is either a plain name, for a component in
/// the same registry, or a `{ registry = "...", name = "..." }` table, for a
/// component in another registry.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// A component in the same registry.
    Same(String),
    /// A component in another registry. The project must also depend on that
    /// registry's crate.
    Other {
        /// The crate name of the other registry.
        registry: String,
        /// The name of the component.
        name: String,
    },
}

/// A component registry, loaded from the registry directory of a crate.
///
/// A registry offers components and themes, each read from a source file in
/// the registry directory.
pub struct Registry {
    dir: PathBuf,
    themes: BTreeMap<String, ThemeEntry>,
    components: BTreeMap<String, Entry>,
}

impl Registry {
    /// Loads the registry whose `registry.toml` is in `dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be read or parsed, or if its
    /// format version is newer than [`MANIFEST_VERSION`].
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

    /// Returns the names of all components in the registry, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }

    /// Looks up a component by name.
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

    /// Returns the names of all themes in the registry, sorted.
    pub fn theme_names(&self) -> impl Iterator<Item = &str> {
        self.themes.keys().map(String::as_str)
    }

    /// Looks up a theme by name.
    #[must_use]
    pub fn theme(&self, name: &str) -> Option<Theme<'_>> {
        self.themes.get_key_value(name).map(|(name, entry)| Theme {
            name,
            entry,
            dir: &self.dir,
        })
    }
}

/// A component in a [`Registry`].
pub struct Component<'a> {
    name: &'a str,
    entry: &'a Entry,
    dir: &'a Path,
}

impl Component<'_> {
    /// Returns the name used to add the component, such as `button`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
    }

    /// Reads the component's source and returns its [`content_hash`].
    ///
    /// # Errors
    ///
    /// Returns an error if the component's source file cannot be read.
    pub fn hash(&self) -> Result<String, Error> {
        Ok(content_hash(&self.read_source()?))
    }

    /// Returns the filename the component is installed as in the project's
    /// components directory, such as `button.rs`.
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

    /// Returns the components that are installed together with this one.
    #[must_use]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.entry.dependencies
    }
}

/// A theme in a [`Registry`].
///
/// A theme is a CSS file. `topcoat ui init` copies it into the project, where
/// it becomes the Tailwind input.
pub struct Theme<'a> {
    name: &'a str,
    entry: &'a ThemeEntry,
    dir: &'a Path,
}

impl Theme<'_> {
    /// Returns the name used to select the theme, such as `neutral`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the filename the theme is installed as in the project.
    ///
    /// This is always `styles.css`, whatever the name of the source file in
    /// the registry.
    #[must_use]
    pub fn file_name(&self) -> &'static str {
        "styles.css"
    }

    /// Reads the theme's source and returns its [`content_hash`].
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

/// Returns the content hash of a component or theme source: `sha256:`
/// followed by the SHA-256 of the source as lowercase hex.
///
/// The same source always gives the same hash. A project records the hash when
/// it installs a component, and a different hash of the registry's current
/// source means that an update is available.
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

/// An error while loading a registry or reading one of its sources.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read.
    #[error("failed to read {path:?}")]
    Read {
        /// The path of the file.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The `registry.toml` manifest is not valid.
    #[error("failed to parse registry manifest")]
    Parse(#[from] toml::de::Error),
    /// The manifest has a format version newer than [`MANIFEST_VERSION`].
    #[error(
        "registry manifest has format version {found}, but this build supports up to {supported}"
    )]
    UnsupportedVersion {
        /// The version in the manifest.
        found: u32,
        /// The newest supported version.
        supported: u32,
    },
}
