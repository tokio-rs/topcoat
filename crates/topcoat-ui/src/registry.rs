use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The manifest file naming the components within a registry.
pub const MANIFEST_FILE: &str = "registry.toml";

/// The registry used when a project does not specify one of its own: the
/// component registry published alongside this repository.
pub const DEFAULT_REGISTRY: &str =
    "https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat-ui/registry";

/// The location of a component registry.
#[derive(Clone, Debug)]
pub enum Source {
    /// A local directory containing the manifest and component files.
    Path(PathBuf),
    /// A remote base URL under which the manifest and component files are served.
    Url(String),
}

impl Source {
    /// Interprets a string as a remote URL if it looks like one (`http://` or
    /// `https://`), otherwise as a local filesystem path. A `file://` prefix
    /// names a local path explicitly.
    pub fn parse(location: &str) -> Self {
        if let Some(path) = location.strip_prefix("file://") {
            Self::Path(PathBuf::from(path))
        } else if location.starts_with("http://") || location.starts_with("https://") {
            Self::Url(location.trim_end_matches('/').to_string())
        } else {
            Self::Path(PathBuf::from(location))
        }
    }

    /// Resolves a dependency's registry location against this source — the
    /// registry that declared the dependency. A relative `file://` location is
    /// resolved against this source's directory (so a dependency can point at a
    /// sibling registry like `file://../other`); absolute `file://` paths and
    /// remote URLs are returned unchanged.
    pub fn resolve(&self, location: &str) -> String {
        let Some(relative) = location.strip_prefix("file://") else {
            return location.to_string();
        };
        if relative.starts_with('/') {
            return location.to_string();
        }
        let Self::Path(base) = self else {
            // A relative local path under a remote registry has no meaningful
            // base; leave it as given.
            return location.to_string();
        };
        let resolved = normalize_lexically(&base.join(relative));
        format!("file://{}", resolved.display())
    }

    /// Resolves a child file (e.g. the manifest or a component file) under this
    /// source.
    fn child(&self, name: &str) -> Source {
        match self {
            Self::Path(base) => Self::Path(base.join(name)),
            Self::Url(base) => Self::Url(format!("{base}/{name}")),
        }
    }

    /// Reads the contents at this exact location as a string.
    async fn read(&self) -> Result<String, Error> {
        match self {
            Self::Path(path) => {
                std::fs::read_to_string(path).map_err(|source| Error::ReadPath {
                    path: path.clone(),
                    source,
                })
            }
            Self::Url(url) => {
                let fetch = |source| Error::Fetch {
                    url: url.clone(),
                    source,
                };
                reqwest::get(url)
                    .await
                    .and_then(reqwest::Response::error_for_status)
                    .map_err(fetch)?
                    .text()
                    .await
                    .map_err(fetch)
            }
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::Url(url) => write!(f, "{url}"),
        }
    }
}

/// The parsed `registry.toml` manifest.
#[derive(Deserialize)]
struct Manifest {
    /// The registry's own name, used when it is added to a project as a
    /// dependency's registry. Every registry must declare one.
    name: String,
    #[serde(default)]
    components: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: String,
    source: String,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}

/// Another component that must be installed alongside a component.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// A component in the same registry, named directly.
    Same(String),
    /// A component in another registry, identified by that registry's location.
    Other { registry: String, name: String },
}

/// A component registry loaded from a [`Source`].
pub struct Registry {
    source: Source,
    name: String,
    components: BTreeMap<String, Entry>,
}

impl Registry {
    /// Loads a registry by reading and parsing its manifest from `source`.
    pub async fn load(source: Source) -> Result<Self, Error> {
        let raw = source.child(MANIFEST_FILE).read().await?;
        let manifest: Manifest = toml::from_str(&raw)?;
        Ok(Self {
            source,
            name: manifest.name,
            components: manifest.components,
        })
    }

    /// The registry's own declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The names of every component in the registry, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }

    /// Looks up a component by its registry name.
    pub fn get(&self, name: &str) -> Option<Component<'_>> {
        self.components.get_key_value(name).map(|(name, entry)| Component {
            name,
            entry,
            source: &self.source,
        })
    }
}

/// A single component within a [`Registry`].
pub struct Component<'a> {
    name: &'a str,
    entry: &'a Entry,
    source: &'a Source,
}

impl Component<'_> {
    /// The name used to add the component, e.g. `button`.
    pub fn name(&self) -> &str {
        self.name
    }

    /// The component's version, recorded per component in the install state so
    /// that updates can be surfaced individually.
    pub fn version(&self) -> &str {
        &self.entry.version
    }

    /// The file name written into the user's components directory.
    pub fn file_name(&self) -> &str {
        Path::new(&self.entry.source)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.entry.source)
    }

    /// Reads the component's Rust source from the registry.
    pub async fn fetch_source(&self) -> Result<String, Error> {
        self.source.child(&self.entry.source).read().await
    }

    /// The other components this component depends on.
    pub fn dependencies(&self) -> &[Dependency] {
        &self.entry.dependencies
    }
}

/// An error loading a registry or one of its components.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path:?}")]
    ReadPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to fetch {url}")]
    Fetch {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("failed to parse registry manifest")]
    Parse(#[from] toml::de::Error),
}

/// Resolves `.` and `..` components in a path lexically (without touching the
/// filesystem), so a resolved registry location is stored in a clean form.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
                if matches!(out.components().next_back(), Some(std::path::Component::Normal(_))) =>
            {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
