use std::{
    collections::{BTreeMap, btree_map::Entry},
    path::{Component, Path, PathBuf},
};

/// The files of a new application, keyed by package-relative path.
#[derive(Default)]
pub struct ProjectPlan {
    files: BTreeMap<PathBuf, String>,
}

impl ProjectPlan {
    /// Adds a file at the package-relative `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if `path` is not a plain relative path or another file has
    /// already been added at it.
    pub fn add(
        &mut self,
        path: impl Into<PathBuf>,
        contents: impl Into<String>,
    ) -> Result<(), String> {
        let path = path.into();
        let plain = path.components().next().is_some()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !plain {
            return Err(format!(
                "generated path {} is not a plain relative path",
                path.display()
            ));
        }

        match self.files.entry(path) {
            Entry::Occupied(entry) => Err(format!(
                "two files are generated at {}",
                entry.key().display()
            )),
            Entry::Vacant(entry) => {
                entry.insert(contents.into());
                Ok(())
            }
        }
    }

    /// The planned files and their contents, sorted by path.
    pub fn files(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.files
            .iter()
            .map(|(path, contents)| (path.as_path(), contents.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_second_file_at_the_same_path() {
        let mut plan = ProjectPlan::default();
        plan.add("src/main.rs", "fn main() {}").unwrap();
        assert!(plan.add("src/main.rs", "fn main() {}").is_err());
        assert!(plan.add("src/./main.rs", "fn main() {}").is_err());
    }

    #[test]
    fn rejects_paths_outside_the_package() {
        let mut plan = ProjectPlan::default();
        for path in ["", "/etc/passwd", "../outside.rs", "src/../../outside.rs"] {
            assert!(plan.add(path, "").is_err(), "{path}");
        }
    }
}
