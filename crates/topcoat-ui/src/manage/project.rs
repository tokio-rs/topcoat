use std::path::{Path, PathBuf};

use super::state::STATE_FILE;

/// The cargo workspace a management operation acts on. Every relative path in
/// the install state (the components directory and installed file paths) is
/// relative to the project root (the directory that holds `components.toml`), so
/// operations behave the same regardless of the working directory.
pub struct Project {
    root: PathBuf,
}

impl Project {
    /// Locates the project root: the cargo workspace root containing `dir` (or
    /// the current directory when `dir` is `None`), falling back to that
    /// directory itself when it is not inside a workspace.
    pub fn locate(dir: Option<PathBuf>) -> Result<Self, String> {
        let start = dir.unwrap_or_else(|| PathBuf::from("."));
        let root = workspace_root(&start).unwrap_or_else(|| start.clone());
        let root = std::fs::canonicalize(&root).map_err(|error| {
            format!("could not resolve project directory {}: {error}", root.display())
        })?;
        Ok(Self { root })
    }

    /// The resolved project root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The path to the install-state file at the project root.
    pub(super) fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    /// Resolves a project-relative path against the project root.
    pub(super) fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

/// The root of the cargo workspace containing `dir`, if it is inside one. Uses
/// `cargo locate-project` so no JSON parsing or async runtime is required.
fn workspace_root(dir: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    Path::new(stdout.trim()).parent().map(Path::to_path_buf)
}
