use std::{io::ErrorKind, path::Path};

use super::plan::ProjectPlan;

/// Writes the files of `plan` into a new directory at `destination`.
///
/// The directory is created before anything is written and must not exist yet, so
/// existing files are never replaced, even if another process creates the destination
/// concurrently. Missing parent directories are created. If writing fails, the new
/// directory is removed again.
///
/// # Errors
///
/// Returns an error if `destination` already exists or a directory or file cannot be
/// created.
pub fn publish(plan: &ProjectPlan, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    match std::fs::create_dir(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(format!("{} already exists", destination.display()));
        }
        Err(error) => {
            return Err(format!(
                "failed to create {}: {error}",
                destination.display()
            ));
        }
    }

    write_files(plan, destination).inspect_err(|_| {
        // The directory was created above, so everything in it was written here.
        let _ = std::fs::remove_dir_all(destination);
    })
}

fn write_files(plan: &ProjectPlan, destination: &Path) -> Result<(), String> {
    for (path, contents) in plan.files() {
        let path = destination.join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        std::fs::write(&path, contents)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new::temp_dir::TempDir;

    fn plan() -> ProjectPlan {
        let mut plan = ProjectPlan::default();
        plan.add("Cargo.toml", "[package]\n").unwrap();
        plan.add("src/main.rs", "fn main() {}\n").unwrap();
        plan
    }

    #[test]
    fn writes_every_file_into_a_new_directory() {
        let temp = TempDir::new("writes");
        let destination = temp.path().join("nested/my-app");
        publish(&plan(), &destination).unwrap();

        for (path, contents) in plan().files() {
            assert_eq!(
                std::fs::read_to_string(destination.join(path)).unwrap(),
                contents
            );
        }
    }

    #[test]
    fn leaves_an_existing_destination_untouched() {
        let temp = TempDir::new("existing");
        let destination = temp.path().join("my-app");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("Cargo.toml"), "keep me").unwrap();

        assert!(publish(&plan(), &destination).is_err());
        assert_eq!(
            std::fs::read_to_string(destination.join("Cargo.toml")).unwrap(),
            "keep me"
        );
        assert!(!destination.join("src").exists());
    }

    #[test]
    fn rejects_an_existing_empty_destination() {
        let temp = TempDir::new("empty");
        let destination = temp.path().join("my-app");
        std::fs::create_dir(&destination).unwrap();

        assert!(publish(&plan(), &destination).is_err());
        assert!(std::fs::read_dir(&destination).unwrap().next().is_none());
    }
}
