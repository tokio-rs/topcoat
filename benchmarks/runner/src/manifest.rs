//! App manifests (`apps/<id>/app.toml`).

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// The order apps appear in when nothing else decides it. Unknown ids follow alphabetically.
const PREFERRED_ORDER: [&str; 11] = [
    "topcoat",
    "axum-maud",
    "leptos",
    "leptos-islands",
    "dioxus",
    "nextjs",
    "rails",
    "aspnet",
    "go-templ",
    "django",
    "phoenix",
];

/// The id of the app every other app is compared against by `verify`.
pub const REFERENCE_APP: &str = "axum-maud";

/// How an app produces its pages and what it ships to the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rendering {
    /// Server-rendered HTML with at most a small script for the interactive parts.
    #[serde(rename = "ssr")]
    Ssr,
    /// Server-rendered HTML that a client runtime hydrates as a whole.
    #[serde(rename = "ssr+hydration")]
    SsrHydration,
    /// Server-rendered HTML where only interactive islands hydrate.
    #[serde(rename = "islands")]
    Islands,
    /// React Server Components with hydration of client components.
    #[serde(rename = "rsc+hydration")]
    RscHydration,
}

impl fmt::Display for Rendering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ssr => "ssr",
            Self::SsrHydration => "ssr+hydration",
            Self::Islands => "islands",
            Self::RscHydration => "rsc+hydration",
        })
    }
}

/// An app whose `app.toml` failed to load, as `(directory name, error)`.
pub type Broken = (String, String);

/// A parsed `app.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub language: String,
    pub kind: String,
    pub rendering: Rendering,
    pub description: String,
    #[serde(default)]
    pub requires: Vec<String>,
    pub version_command: Option<String>,
    /// Files or directories whose total size is reported as the deployable artifact size.
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub build: Build,
    pub run: Run,
    #[serde(default)]
    pub client: Client,
    /// The app directory. Not part of the file.
    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub command: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_ready_timeout")]
    pub ready_timeout_secs: u64,
    #[serde(default = "default_stop_timeout")]
    pub stop_timeout_secs: u64,
}

fn default_ready_timeout() -> u64 {
    60
}

fn default_stop_timeout() -> u64 {
    10
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    #[serde(default)]
    pub extra_assets: Vec<String>,
}

impl Manifest {
    /// Loads every `apps/*/app.toml` under `root`, in the preferred display order.
    /// Manifests that fail to load are returned separately as `(directory name, error)`.
    pub fn discover(root: &Path) -> Result<(Vec<Self>, Vec<Broken>)> {
        let apps_dir = root.join("apps");
        let mut manifests = Vec::new();
        let mut broken = Vec::new();
        let Ok(entries) = std::fs::read_dir(&apps_dir) else {
            return Ok((manifests, broken));
        };
        for entry in entries {
            let dir = entry?.path();
            let path = dir.join("app.toml");
            if path.is_file() {
                match Self::load(&path) {
                    Ok(manifest) => manifests.push(manifest),
                    Err(error) => {
                        let id = dir
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned());
                        broken.push((id.unwrap_or_default(), format!("{error:#}")));
                    }
                }
            }
        }
        broken.sort();
        manifests.sort_by_key(|manifest| {
            let rank = PREFERRED_ORDER
                .iter()
                .position(|id| *id == manifest.id)
                .unwrap_or(PREFERRED_ORDER.len());
            (rank, manifest.id.clone())
        });
        Ok((manifests, broken))
    }

    /// Like [`Manifest::discover`], but warns about broken manifests and skips them. A broken
    /// manifest is an error only if its app was requested explicitly in `apps`.
    pub fn discover_selected(root: &Path, apps: &[String]) -> Result<Vec<Self>> {
        let (manifests, broken) = Self::discover(root)?;
        for (id, error) in broken {
            if apps.contains(&id) {
                bail!("cannot load the manifest of `{id}`: {error}");
            }
            crate::run::warn(&format!(
                "skipping `{id}`, its manifest is invalid: {error}"
            ));
        }
        Ok(manifests)
    }

    fn load(path: &Path) -> Result<Self> {
        let text = crate::read_file(path)?;
        let mut manifest: Self =
            toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))?;
        let dir = path.parent().expect("manifest path has a parent");
        let dir_name = dir.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if manifest.id != dir_name {
            bail!(
                "{}: id `{}` does not match the directory name `{dir_name}`",
                path.display(),
                manifest.id
            );
        }
        manifest.dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        Ok(manifest)
    }

    pub fn is_rust(&self) -> bool {
        self.kind == "rust"
    }

    /// The cargo profile Rust apps build with.
    pub fn profile(&self) -> &'static str {
        if self.is_rust() {
            "benchmark"
        } else {
            "release"
        }
    }

    /// Substitutes `{app_dir}`, `{profile}`, `{features}` and `{allocator}` in a manifest
    /// command, environment value or path.
    pub fn expand(&self, command: &str, config: &Config) -> String {
        let (features, allocator) = if self.is_rust() {
            (config.allocator.cargo_features(), config.allocator.name())
        } else {
            ("", "default")
        };
        command
            .replace("{app_dir}", &self.dir.display().to_string())
            .replace("{profile}", self.profile())
            .replace("{features}", features)
            .replace("{allocator}", allocator)
    }

    /// Commands from `requires` that are not on `PATH`.
    pub fn missing_requirements(&self) -> Vec<String> {
        self.requires
            .iter()
            .filter(|command| find_on_path(command).is_none())
            .cloned()
            .collect()
    }

    /// The total size of `artifacts`, if any are listed and all exist.
    pub fn artifact_bytes(&self, config: &Config) -> Option<u64> {
        if self.artifacts.is_empty() {
            return None;
        }
        let mut total = 0;
        for artifact in &self.artifacts {
            total += path_size(&self.dir.join(self.expand(artifact, config)))?;
        }
        Some(total)
    }
}

/// Returns the full path of `command` if it is an executable on `PATH`.
pub fn find_on_path(command: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let is_executable = |path: &Path| {
        path.metadata()
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    };
    if command.contains('/') {
        let path = PathBuf::from(command);
        return is_executable(&path).then_some(path);
    }
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(command))
        .find(|path| is_executable(path))
}

fn path_size(path: &Path) -> Option<u64> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.is_dir() {
        let mut total = 0;
        for entry in std::fs::read_dir(path).ok()? {
            total += path_size(&entry.ok()?.path()).unwrap_or(0);
        }
        Some(total)
    } else {
        Some(meta.len())
    }
}
