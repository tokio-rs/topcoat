use std::path::{Path, PathBuf};

use super::state::STATE_FILE;

/// The cargo workspace a management operation acts on. Every relative path in
/// the install state — registry `file://` locations, component directories, and
/// installed file paths — is relative to the project root (the directory that
/// holds `components.toml`), so operations behave the same regardless of the
/// working directory.
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

    /// Converts a stored registry location into one usable from any working
    /// directory: a relative `file://` location is made absolute against the
    /// project root; remote URLs and absolute paths are left unchanged.
    pub(super) fn to_working(&self, url: &str) -> String {
        let Some(rest) = url.strip_prefix("file://") else {
            return url.to_string();
        };
        let rest = rest.strip_prefix("./").unwrap_or(rest);
        if Path::new(rest).is_absolute() {
            return url.to_string();
        }
        format!("file://{}", self.root.join(rest).display())
    }

    /// Converts a registry location into the form stored in `components.toml`: a
    /// `file://` path inside the project is made relative to the project root;
    /// remote URLs and paths outside the project are left as they are.
    pub(super) fn to_stored(&self, url: &str) -> String {
        let Some(rest) = url.strip_prefix("file://") else {
            return url.to_string();
        };
        let path = Path::new(rest);
        if path.is_absolute() {
            return match path.strip_prefix(&self.root) {
                Ok(relative) => format!("file://{}", relative.display()),
                Err(_) => url.to_string(),
            };
        }
        let rest = rest.strip_prefix("./").unwrap_or(rest);
        format!("file://{rest}")
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
