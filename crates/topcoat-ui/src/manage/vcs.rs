use std::{path::Path, process::Command};

/// The version-control state of a directory.
pub(super) enum Vcs {
    /// The directory is in no git repository, or git is not installed.
    None,
    /// The directory is in a git repository.
    Git {
        /// The files with uncommitted changes. Staged changes and untracked files
        /// count, and ignored files do not.
        dirty: Vec<String>,
    },
}

impl Vcs {
    /// Reads the state of the repository that contains `dir`.
    pub(super) fn inspect(dir: &Path) -> Self {
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output();
        match status {
            // Each line holds a two-letter status, a space, and the path.
            Ok(output) if output.status.success() => Self::Git {
                dirty: String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| line.get(3..))
                    .map(str::to_string)
                    .collect(),
            },
            _ => Self::None,
        }
    }

    /// Checks that a merge into a file with local changes can be undone.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no repository and `allow_no_vcs` is `false`, or if
    /// the repository has uncommitted changes and `allow_dirty` is `false`.
    pub(super) fn ensure_undoable(
        &self,
        allow_dirty: bool,
        allow_no_vcs: bool,
    ) -> Result<(), String> {
        match self {
            Self::None if allow_no_vcs => Ok(()),
            Self::None => Err(String::from(
                "no git repository found for this package, so a merge into your changes could \
                 not be undone; create one and commit, or pass `--allow-no-vcs` to merge anyway",
            )),
            Self::Git { dirty } if allow_dirty || dirty.is_empty() => Ok(()),
            Self::Git { dirty } => {
                let mut error = String::from(
                    "the repository has uncommitted changes, so a merge into your changes could \
                     not be undone cleanly; commit or stash them, or pass `--allow-dirty` to \
                     merge anyway:\n",
                );
                for file in dirty {
                    error.push_str("\n  * ");
                    error.push_str(file);
                }
                Err(error)
            }
        }
    }
}
