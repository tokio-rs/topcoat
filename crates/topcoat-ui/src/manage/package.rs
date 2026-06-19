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
    /// Locates the crate root: the cargo crate root containing `dir` (or the
    /// current directory when `dir` is `None`), falling back to that directory
    /// itself when it is not inside a crate.
    pub fn locate(dir: Option<PathBuf>) -> Result<Self, String> {
        let start = dir.unwrap_or_else(|| PathBuf::from("."));
        let root = crate_root(&start).unwrap_or_else(|| start.clone());
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
