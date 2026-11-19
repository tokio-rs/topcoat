use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

use super::package::Package;
use crate::{DEFAULT_REGISTRY, DEFAULT_REGISTRY_CRATE, Registry};

/// The Cargo dependency graph used to resolve component registries.
///
/// Custom registries must be direct dependencies. The built-in registry uses
/// [`DEFAULT_REGISTRY`] as its name and may be a transitive dependency.
pub(super) struct Workspace {
    /// Every resolved package, indexed by crate name.
    packages: HashMap<String, MetadataPackage>,
    /// The names of crates declared as direct dependencies by any workspace
    /// member; the crates a registry may be referenced from.
    direct_deps: HashSet<String>,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetadataPackage>,
    /// Package ids of the workspace's own members.
    workspace_default_members: Vec<String>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    id: String,
    version: String,
    /// Where Cargo got the package from. Path dependencies have no source.
    #[serde(default)]
    source: Option<String>,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
    /// The crate's `[package.metadata]` table, holding the
    /// `[package.metadata.topcoat-ui]` registry declaration when present.
    #[serde(default)]
    metadata: serde_json::Value,
}

impl MetadataPackage {
    /// The index of the package registry the package was published to, in the form
    /// Cargo accepts as a dependency's `registry-index`. Path and git dependencies
    /// have none.
    fn registry_index(&self) -> Option<&str> {
        let source = self.source.as_deref()?;
        source
            .strip_prefix("registry+")
            .or_else(|| source.starts_with("sparse+").then_some(source))
    }
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
}

impl Workspace {
    /// Resolves the package's dependency graph by running `cargo metadata` at the
    /// package root.
    pub(super) fn load(package: &Package) -> Result<Self, String> {
        Ok(Self::from_metadata(cargo_metadata(package, None)?))
    }

    /// Indexes the packages and direct dependencies of a resolved dependency graph.
    fn from_metadata(metadata: Metadata) -> Self {
        let members: HashSet<&str> = metadata
            .workspace_default_members
            .iter()
            .map(String::as_str)
            .collect();
        let direct_deps = metadata
            .packages
            .iter()
            .filter(|package| members.contains(package.id.as_str()))
            .flat_map(|package| package.dependencies.iter().map(|dep| dep.name.clone()))
            .collect();
        let packages = metadata
            .packages
            .into_iter()
            .map(|package| (package.name.clone(), package))
            .collect();

        Self {
            packages,
            direct_deps,
        }
    }

    /// Returns the version of a registry's crate if it was published to a package
    /// registry. Path and git dependencies have none, because their version does not
    /// identify their sources.
    pub(super) fn registry_version(&self, name: &str) -> Option<&str> {
        let package = self.packages.get(registry_crate(name))?;
        package.registry_index()?;
        Some(&package.version)
    }

    /// Loads a registry as it was published in `version` of its crate. Cargo downloads
    /// that version if it is not cached.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry's crate does not come from a package registry,
    /// Cargo cannot provide the version, or the version does not hold a loadable
    /// registry.
    #[expect(dead_code, reason = "no command reads published versions yet")]
    pub(super) fn published_registry(
        &self,
        package: &Package,
        name: &str,
        version: &str,
    ) -> Result<Registry, String> {
        let crate_name = registry_crate(name);
        let index = self
            .packages
            .get(crate_name)
            .and_then(MetadataPackage::registry_index)
            .ok_or_else(|| {
                format!("registry crate `{crate_name}` does not come from a package registry")
            })?;

        // Resolve a throwaway package that depends on exactly that version. It is its
        // own workspace, so Cargo does not look for one in the directories above it.
        let dir = ScratchDir::create()?;
        let manifest = dir.0.join("Cargo.toml");
        let write = |path: &Path, contents: &str| {
            std::fs::write(path, contents)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))
        };
        write(
            &manifest,
            &format!(
                "[package]\nname = \"topcoat-ui-base\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
                 [lib]\npath = \"lib.rs\"\n\n\
                 [workspace]\n\n\
                 [dependencies.{crate_name}]\nversion = {}\nregistry-index = {}\n",
                toml::Value::from(format!("={version}")),
                toml::Value::from(index),
            ),
        )?;
        write(&dir.0.join("lib.rs"), "")?;

        let metadata = cargo_metadata(package, Some(&manifest))?;
        let published = metadata
            .packages
            .iter()
            .find(|package| package.name == crate_name && package.version == version)
            .ok_or_else(|| format!("Cargo did not resolve `{crate_name}` {version}"))?;
        Registry::load(registry_path(published, crate_name)?).map_err(|error| {
            format!("failed to load registry `{name}` from `{crate_name}` {version}: {error}")
        })
    }

    /// Finds a registry's manifest directory and checks that it is an allowed
    /// dependency. The built-in registry may be transitive.
    pub(super) fn registry_dir(&self, name: &str) -> Result<PathBuf, String> {
        if name == DEFAULT_REGISTRY {
            let package = self.packages.get(DEFAULT_REGISTRY_CRATE).ok_or_else(|| {
                format!(
                    "the built-in registry crate `{DEFAULT_REGISTRY_CRATE}` is not in the \
                     dependency graph; enable the `ui` feature on `topcoat`"
                )
            })?;
            return registry_path(package, DEFAULT_REGISTRY_CRATE);
        }

        let package = self.packages.get(name).ok_or_else(|| {
            format!("registry crate `{name}` is not in the dependency graph; add it to Cargo.toml")
        })?;

        if !self.direct_deps.contains(name) {
            return Err(format!(
                "registry crate `{name}` must be a direct dependency in Cargo.toml"
            ));
        }

        registry_path(package, name)
    }

    /// Returns allowed registry names in sorted order, including the built-in registry
    /// when available.
    pub(super) fn available_registries(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .packages
            .values()
            .filter(|package| {
                package.name != DEFAULT_REGISTRY_CRATE
                    && self.direct_deps.contains(&package.name)
                    && registry_subdir(package).is_some()
            })
            .map(|package| package.name.clone())
            .collect();
        if self
            .packages
            .get(DEFAULT_REGISTRY_CRATE)
            .is_some_and(|package| registry_subdir(package).is_some())
        {
            names.push(DEFAULT_REGISTRY.to_string());
        }
        names.sort();
        names.dedup();
        names
    }
}

/// A new directory in the system's temporary directory. It is removed when dropped.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn create() -> Result<Self, String> {
        // The process ID and the time make the name unique. Creating fails rather
        // than reusing a directory that already exists.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let name = format!("topcoat-ui-{}-{nanos}", std::process::id());
        let path = std::env::temp_dir().join(name);
        std::fs::create_dir(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // Best effort: a leftover directory is harmless.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The crate that provides the registry `name`.
fn registry_crate(name: &str) -> &str {
    if name == DEFAULT_REGISTRY {
        DEFAULT_REGISTRY_CRATE
    } else {
        name
    }
}

/// Runs `cargo metadata` from the package root, so the package's Cargo configuration
/// applies. Resolves `manifest` if given, and the package itself otherwise.
fn cargo_metadata(package: &Package, manifest: Option<&Path>) -> Result<Metadata, String> {
    let mut command = Command::new("cargo");
    command
        .args(["metadata", "--format-version=1"])
        .current_dir(package.root());
    if let Some(manifest) = manifest {
        command.arg("--manifest-path").arg(manifest);
    }
    let output = command
        .output()
        .map_err(|error| format!("failed to run `cargo metadata`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("failed to parse `cargo metadata` output: {error}"))
}

/// The registry directory `package` declares (the directory holding
/// `registry.toml`), resolved against the crate root.
fn registry_path(package: &MetadataPackage, name: &str) -> Result<PathBuf, String> {
    let registry = registry_subdir(package).ok_or_else(|| {
        format!(
            "crate `{name}` is not a topcoat-ui registry \
             (missing `[package.metadata.topcoat-ui].registry`)"
        )
    })?;
    let crate_root = package
        .manifest_path
        .parent()
        .expect("a manifest path always has a parent directory");
    Ok(crate_root.join(registry))
}

/// The registry directory a crate declares via `[package.metadata.topcoat-ui]`,
/// if any.
fn registry_subdir(package: &MetadataPackage) -> Option<&str> {
    package
        .metadata
        .get("topcoat-ui")?
        .get("registry")?
        .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dependency graph holding one registry crate that Cargo got from `source`.
    fn workspace(name: &str, source: Option<&str>) -> Workspace {
        let metadata = serde_json::json!({
            "workspace_default_members": [],
            "packages": [{
                "name": name,
                "id": name,
                "version": "1.2.3",
                "source": source,
                "manifest_path": "/registry/Cargo.toml",
                "dependencies": [],
            }],
        });
        Workspace::from_metadata(serde_json::from_value(metadata).unwrap())
    }

    #[test]
    fn published_registries_have_a_version() {
        let crates_io = Some("registry+https://github.com/rust-lang/crates.io-index");
        let builtin = workspace(DEFAULT_REGISTRY_CRATE, crates_io);
        assert_eq!(builtin.registry_version(DEFAULT_REGISTRY), Some("1.2.3"));

        let sparse = workspace("custom", Some("sparse+https://example.com/index/"));
        assert_eq!(sparse.registry_version("custom"), Some("1.2.3"));
    }

    #[test]
    fn path_and_git_registries_have_no_version() {
        let path = workspace("custom", None);
        assert_eq!(path.registry_version("custom"), None);

        let git = workspace("custom", Some("git+https://example.com/custom.git#0123abc"));
        assert_eq!(git.registry_version("custom"), None);
    }
}
