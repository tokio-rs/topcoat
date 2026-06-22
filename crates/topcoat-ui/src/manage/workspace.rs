use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::package::Package;

/// The cargo dependency graph of a package, used to resolve registries.
///
/// A registry is a crate, referenced by name. To be used as a registry a crate
/// must (a) be reachable in the package's dependency graph and (b) be a *direct*
/// dependency declared in the package's `Cargo.toml`. The built-in `topcoat`
/// registry is no exception: every Topcoat project depends on `topcoat` directly,
/// so it satisfies these rules like any other registry.
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

        Ok(Self {
            packages,
            direct_deps,
        })
    }

    /// Resolves a registry crate name to its registry directory (the directory
    /// holding `registry.toml`), enforcing the dependency rules.
    pub(super) fn registry_dir(&self, crate_name: &str) -> Result<PathBuf, String> {
        let package = self.packages.get(crate_name).ok_or_else(|| {
            format!(
                "registry crate `{crate_name}` is not in the dependency graph; add it to Cargo.toml"
            )
        })?;

        if !self.direct_deps.contains(crate_name) {
            return Err(format!(
                "registry crate `{crate_name}` must be a direct dependency in Cargo.toml"
            ));
        }

        let registry = registry_subdir(package).ok_or_else(|| {
            format!(
                "crate `{crate_name}` is not a topcoat-ui registry \
                 (missing `[package.metadata.topcoat-ui].registry`)"
            )
        })?;

        let crate_root = package
            .manifest_path
            .parent()
            .expect("a manifest path always has a parent directory");
        Ok(crate_root.join(registry))
    }

    /// The crate names of every registry the package may add from: every direct
    /// dependency that declares a registry (including the default `topcoat`
    /// crate), sorted.
    pub(super) fn available_registries(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .packages
            .values()
            .filter(|package| {
                self.direct_deps.contains(&package.name) && registry_subdir(package).is_some()
            })
            .map(|package| package.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }
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
