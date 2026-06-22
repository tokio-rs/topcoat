use std::path::{Path, PathBuf};

use super::state::STATE_FILE;

/// The cargo crate a management operation acts on. Every relative path in the
/// install state (the components directory and installed file paths) is relative
/// to the crate root (the directory that holds `components.toml`), so operations
/// behave the same regardless of the working directory.
pub struct Package {
    root: PathBuf,
}

impl Package {
    /// Locates the crate root. When `package` names a workspace member (like
    /// `cargo -p`), its manifest directory is used; otherwise the cargo crate
    /// root containing the current directory is used, falling back to the
    /// current directory itself when it is not inside a crate.
    pub fn locate(package: Option<String>) -> Result<Self, String> {
        let root = match package {
            Some(name) => package_root(&name)?,
            None => {
                let start = PathBuf::from(".");
                crate_root(&start).unwrap_or(start)
            }
        };
        let root = std::fs::canonicalize(&root).map_err(|error| {
            format!(
                "could not resolve package directory {}: {error}",
                root.display()
            )
        })?;
        Ok(Self { root })
    }

    /// The resolved package root directory.
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
