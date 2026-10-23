use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use console::style;

/// Makes cargo rebuild a package when a file is added to it.
///
/// Cargo rebuilds a crate only when a file that its last build read changes. A macro
/// that reads every file in a directory cannot tell cargo about a file that did not
/// exist yet, so cargo would consider the crate up to date. When a file appears in a
/// package, this sets the modification time of the package's crate roots to now,
/// which makes cargo rebuild the package and run its macros again.
pub struct NewFiles {
    /// The files that existed when watching started, and every file seen since.
    known: HashSet<PathBuf>,
    packages: Vec<Package>,
    /// The crate roots that were touched, with the modification time they were given,
    /// so the events caused by touching them are not reported as changes.
    touched: HashMap<PathBuf, SystemTime>,
}

impl NewFiles {
    /// Starts tracking new files in `packages`, given the files that already exist.
    pub fn new(existing: impl IntoIterator<Item = PathBuf>, packages: Vec<Package>) -> Self {
        Self {
            known: existing.into_iter().collect(),
            packages,
            touched: HashMap::new(),
        }
    }

    /// Records an event for `path` and returns whether it is a change.
    ///
    /// When `path` is a file that was not seen before, the crate roots of its package
    /// are touched. The events caused by touching them are not changes.
    pub fn observe(&mut self, path: &Path) -> bool {
        if let Some(touched) = self.touched.get(path)
            && modified(path).as_ref() == Some(touched)
        {
            return false;
        }
        if path.is_file() && self.known.insert(path.to_path_buf()) {
            self.touch_package_of(path);
        }
        true
    }

    /// Touches the crate roots of the package containing `path`.
    ///
    /// The deepest package wins, so a package nested in another one is touched
    /// instead of the one that contains it.
    fn touch_package_of(&mut self, path: &Path) {
        let Some(package) = self
            .packages
            .iter()
            .filter(|package| path.starts_with(&package.dir))
            .max_by_key(|package| package.dir.components().count())
        else {
            return;
        };
        for root in &package.crate_roots {
            match touch(root) {
                Ok(time) => {
                    self.touched.insert(root.clone(), time);
                }
                Err(error) => eprintln!(
                    "  {}",
                    style(format!("failed to touch {}: {error}", root.display())).yellow()
                ),
            }
        }
    }
}

/// A local package and the root source files of its library and binary crates.
pub struct Package {
    /// The directory of the package's manifest.
    pub dir: PathBuf,
    /// The root source files of the package's library and binary crates.
    pub crate_roots: Vec<PathBuf>,
}

/// Sets the modification time of the file at `path` to now, without changing its
/// contents, and returns the time the file system stored.
fn touch(path: &Path) -> io::Result<SystemTime> {
    let file = File::options().write(true).open(path)?;
    file.set_modified(SystemTime::now())?;
    file.metadata()?.modified()
}

/// The modification time of the file at `path`, if it can be read.
fn modified(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use super::*;

    /// A temporary package with a `src/main.rs` and an existing `src/en.ftl`.
    struct Fixture {
        dir: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("topcoat-new-files-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("src")).unwrap();
            fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
            fs::write(dir.join("src/en.ftl"), "title = Welcome\n").unwrap();
            // An old modification time, so a touch is visible.
            for file in ["src/main.rs", "src/en.ftl"] {
                File::options()
                    .write(true)
                    .open(dir.join(file))
                    .unwrap()
                    .set_modified(SystemTime::UNIX_EPOCH)
                    .unwrap();
            }
            Self { dir }
        }

        fn new_files(&self) -> NewFiles {
            NewFiles::new(
                [self.path("src/main.rs"), self.path("src/en.ftl")],
                vec![Package {
                    dir: self.dir.clone(),
                    crate_roots: vec![self.path("src/main.rs")],
                }],
            )
        }

        fn path(&self, file: &str) -> PathBuf {
            self.dir.join(file)
        }

        /// Whether `src/main.rs` was touched since the fixture was created.
        fn main_touched(&self) -> bool {
            modified(&self.path("src/main.rs")).unwrap() > SystemTime::UNIX_EPOCH
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn a_new_file_touches_the_crate_roots_of_its_package() {
        let fixture = Fixture::new("new");
        let mut new_files = fixture.new_files();
        let ja = fixture.path("src/ja.ftl");
        fs::write(&ja, "title = Yokoso\n").unwrap();
        assert!(new_files.observe(&ja));
        assert!(fixture.main_touched());
        assert_eq!(
            fs::read_to_string(fixture.path("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
    }

    #[test]
    fn a_changed_existing_file_touches_nothing() {
        let fixture = Fixture::new("existing");
        let mut new_files = fixture.new_files();
        let en = fixture.path("src/en.ftl");
        fs::write(&en, "title = Hello\n").unwrap();
        assert!(new_files.observe(&en));
        assert!(!fixture.main_touched());
    }

    #[test]
    fn a_new_file_touches_the_crate_roots_only_once() {
        let fixture = Fixture::new("once");
        let mut new_files = fixture.new_files();
        let ja = fixture.path("src/ja.ftl");
        fs::write(&ja, "title = Yokoso\n").unwrap();
        new_files.observe(&ja);
        let main = fixture.path("src/main.rs");
        File::options()
            .write(true)
            .open(&main)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
            .unwrap();
        assert!(new_files.observe(&ja));
        assert_eq!(
            modified(&main).unwrap(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1)
        );
    }

    #[test]
    fn touching_a_crate_root_is_not_a_change() {
        let fixture = Fixture::new("touch");
        let mut new_files = fixture.new_files();
        let ja = fixture.path("src/ja.ftl");
        fs::write(&ja, "title = Yokoso\n").unwrap();
        new_files.observe(&ja);
        let main = fixture.path("src/main.rs");
        assert!(!new_files.observe(&main));
        // Editing the crate root afterwards is a change again.
        fs::write(&main, "fn main() { println!(); }\n").unwrap();
        File::options()
            .write(true)
            .open(&main)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
            .unwrap();
        assert!(new_files.observe(&main));
    }

    #[test]
    fn a_new_file_outside_every_package_touches_nothing() {
        let fixture = Fixture::new("outside");
        let mut new_files = NewFiles::new(
            [fixture.path("src/main.rs")],
            vec![Package {
                dir: fixture.path("other"),
                crate_roots: vec![fixture.path("src/main.rs")],
            }],
        );
        let ja = fixture.path("src/ja.ftl");
        fs::write(&ja, "title = Yokoso\n").unwrap();
        assert!(new_files.observe(&ja));
        assert!(!fixture.main_touched());
    }
}
