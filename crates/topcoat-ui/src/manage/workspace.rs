use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
};

use serde::Deserialize;

use super::package::Package;
use crate::{DEFAULT_REGISTRY, DEFAULT_REGISTRY_CRATE};

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

#[derive(Deserialize)]
struct Dependency {
    name: String,
}

impl Workspace {
    /// Resolves the package's dependency graph by running `cargo metadata` at the
    /// package root.
    pub(super) fn load(package: &Package) -> Result<Self, String> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version=1"])
            .current_dir(package.root())
            .output()
            .map_err(|error| format!("failed to run `cargo metadata`: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "`cargo metadata` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let metadata: Metadata = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("failed to parse `cargo metadata` output: {error}"))?;
        Ok(Self::from_metadata(metadata))
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
        let crate_name = if name == DEFAULT_REGISTRY {
            DEFAULT_REGISTRY_CRATE
        } else {
            name
        };
        let package = self.packages.get(crate_name)?;
        let source = package.source.as_deref()?;
        (source.starts_with("registry+") || source.starts_with("sparse+"))
            .then_some(package.version.as_str())
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
