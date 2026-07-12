use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use crate::{DEFAULT_REGISTRY, DEFAULT_REGISTRY_CRATE};

use super::package::Package;

/// The cargo dependency graph of a package, used to resolve registries.
///
/// A registry is a crate, referenced by name. To be used as a registry a crate
/// must (a) be reachable in the package's dependency graph and (b) be a *direct*
/// dependency declared in the package's `Cargo.toml`. The built-in registry is
/// the one exception: it is named by the alias [`DEFAULT_REGISTRY`], provided by
/// the [`DEFAULT_REGISTRY_CRATE`] crate, and pulled in transitively by the
/// `topcoat` facade's `ui` feature, so it need not be a direct dependency.
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

    /// Resolves a registry name to its registry directory (the directory holding
    /// `registry.toml`), enforcing the dependency rules.
    ///
    /// The built-in registry is named by the alias [`DEFAULT_REGISTRY`]: it
    /// resolves to the [`DEFAULT_REGISTRY_CRATE`] crate and, since that crate is
    /// pulled in transitively, is exempt from the direct-dependency rule.
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

    /// The names of every registry the package may add from, sorted: the built-in
    /// registry under its alias [`DEFAULT_REGISTRY`] whenever its crate is
    /// reachable, plus every direct dependency that declares a registry.
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
