use std::path::{Path, PathBuf};

use super::state::STATE_FILE;

/// The Cargo package that the `topcoat ui` commands work on.
///
/// Every relative path in `components.toml` is relative to the package root,
/// the directory that holds `components.toml`. So the commands behave the same
/// whatever the working directory is.
pub struct Package {
    root: PathBuf,
}

impl Package {
    /// Finds the package root.
    ///
    /// When `package` names a workspace member, like `cargo -p`, its manifest
    /// directory is the root. Otherwise the root is the package that contains
    /// the current directory, or the current directory itself if it is not
    /// inside a package.
    ///
    /// # Errors
    ///
    /// Returns an error if `cargo metadata` fails, if the named package is not
    /// in the workspace, or if the root cannot be canonicalized.
    pub fn locate(package: Option<String>) -> Result<Self, String> {
        let root = if let Some(name) = package {
            package_root(&name)?
        } else {
            let start = PathBuf::from(".");
            crate_root(&start).unwrap_or(start)
        };
        let root = std::fs::canonicalize(&root).map_err(|error| {
            format!(
                "could not resolve package directory {}: {error}",
                root.display()
            )
        })?;
        Ok(Self { root })
    }

    /// Returns the package root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The path to the install-state file at the package root.
    pub(super) fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    /// Resolves a package-relative path against the package root.
    pub(super) fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

/// The manifest directory of the workspace member named `name`, resolved via
/// `cargo metadata` (mirroring how `cargo -p <SPEC>` selects a package).
fn package_root(name: &str) -> Result<PathBuf, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|error| format!("failed to run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("could not parse cargo metadata: {error}"))?;

    let package = metadata
        .packages
        .iter()
        .find(|package| package.name == name)
        .ok_or_else(|| {
            let available = metadata
                .packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("package `{name}` not found in workspace (available: {available})")
        })?;

    package
        .manifest_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("package `{name}` has no manifest directory"))
}

/// The subset of `cargo metadata` output we read to map a package name to its
/// manifest directory.
#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<MetadataPackage>,
}

#[derive(serde::Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}

/// The root of the cargo crate containing `dir`, if it is inside one. Uses
/// `cargo locate-project` so no JSON parsing or async runtime is required.
fn crate_root(dir: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("cargo")
        .args(["locate-project", "--message-format", "plain"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    Path::new(stdout.trim()).parent().map(Path::to_path_buf)
}
