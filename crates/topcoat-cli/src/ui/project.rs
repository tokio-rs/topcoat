use std::path::{Path, PathBuf};

use clap::Args;

use super::state::STATE_FILE;

/// The `--project` selector shared by the `ui` subcommands.
#[derive(Args)]
pub(super) struct ProjectArg {
    /// Cargo workspace to operate on; its root holds `components.toml`
    /// (defaults to the current workspace)
    #[arg(long)]
    project: Option<PathBuf>,
}

/// The cargo workspace a `ui` command operates on. Every relative path in the
/// install state — registry `file://` locations, component directories, and
/// installed file paths — is relative to the project root (the directory that
/// holds `components.toml`), so the commands behave the same regardless of the
/// working directory.
pub(super) struct Project {
    root: PathBuf,
}

impl Project {
    /// Locates the project root: the cargo workspace root containing the
    /// `--project` directory (or the current directory), falling back to that
    /// directory itself when it is not inside a workspace.
    pub(super) async fn locate(arg: ProjectArg) -> Result<Self, String> {
        let start = arg.project.unwrap_or_else(|| PathBuf::from("."));
        let root = crate::cargo::workspace_root(&start)
            .await
            .unwrap_or_else(|| start.clone());
        let root = std::fs::canonicalize(&root)
            .map_err(|error| format!("could not resolve project directory {}: {error}", root.display()))?;
        Ok(Self { root })
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
