use std::{io::ErrorKind, path::Path, process::Command};

/// The outcome of [`init`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitInit {
    /// A new repository was created.
    Created,
    /// The directory is inside an existing work tree, so no repository was created.
    Existing,
}

/// Initializes a Git repository in the existing directory `path`, unless it is already
/// inside a Git work tree.
///
/// # Errors
///
/// Returns an error if Git is not installed or fails.
pub fn init(path: &Path) -> Result<GitInit, String> {
    let inside = git(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|error| describe(&error))?;
    if inside.status.success() && inside.stdout.trim_ascii() == b"true" {
        return Ok(GitInit::Existing);
    }

    let output = git(path)
        .args(["init", "--quiet"])
        .output()
        .map_err(|error| describe(&error))?;
    if output.status.success() {
        Ok(GitInit::Created)
    } else {
        Err(String::from_utf8_lossy(output.stderr.trim_ascii()).into_owned())
    }
}

/// A Git command run in `path`. Variables that point Git at another repository are
/// cleared, so the command only sees the repository containing `path`, if any.
fn git(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

fn describe(error: &std::io::Error) -> String {
    if error.kind() == ErrorKind::NotFound {
        "git is not installed".to_string()
    } else {
        format!("failed to run git: {error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new::temp_dir::TempDir;

    #[test]
    fn creates_a_repository_outside_a_work_tree() {
        let temp = TempDir::new("git-create");
        assert_eq!(init(temp.path()).unwrap(), GitInit::Created);
        assert!(temp.path().join(".git").exists());
    }

    #[test]
    fn does_not_nest_a_repository_inside_a_work_tree() {
        let temp = TempDir::new("git-nested");
        init(temp.path()).unwrap();
        let app = temp.path().join("my-app");
        std::fs::create_dir(&app).unwrap();

        assert_eq!(init(&app).unwrap(), GitInit::Existing);
        assert!(!app.join(".git").exists());
    }
}
