use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io,
    path::{Path, PathBuf},
    slice,
    sync::Arc,
    time::SystemTime,
};

use console::style;

use super::{PathFilter, existing_paths};

/// Triggers a package rebuild when a file is added.
///
/// Cargo tracks files read during a build, but cannot track files that do not exist
/// yet. A macro that searches a directory may therefore miss new files on later builds.
/// This tracker updates the modification times of the package's source roots so Cargo
/// runs its macros again.
///
/// Moving a file or a directory of files into a package also counts as adding files,
/// even if they were there before.
pub struct NewFiles {
    filter: Arc<PathFilter>,
    /// Paths known to exist from the initial scan and later events.
    known: HashSet<PathBuf>,
    packages: Vec<Package>,
    /// Source roots touched by this tracker and their new modification times. Used to
    /// ignore the resulting events.
    touched: HashMap<PathBuf, SystemTime>,
}

impl NewFiles {
    /// Tracks files under `roots` for the given packages, skipping paths rejected by
    /// `filter`.
    pub fn new(roots: &[PathBuf], filter: Arc<PathFilter>, packages: Vec<Package>) -> Self {
        Self {
            known: existing_paths(roots, &filter).into_iter().collect(),
            filter,
            packages,
            touched: HashMap::new(),
        }
    }

    /// Processes an event for `path` and reports whether it should trigger a rebuild.
    ///
    /// A new file, or a new directory containing files, causes the package's source
    /// roots to be touched. Events from those touches are ignored.
    pub fn observe(&mut self, path: &Path) -> bool {
        if let Some(touched) = self.touched.get(path)
            && modified(path).as_ref() == Some(touched)
        {
            return false;
        }
        if !path.exists() {
            // A path that is moved away or deleted is new again when it comes back.
            self.known.retain(|known| !known.starts_with(path));
        } else if self.known.insert(path.to_path_buf()) && self.adds_files(path) {
            self.touch_package_of(path);
        }
        true
    }

    /// Checks whether `path` adds any files. For a directory, also records its contents
    /// as known paths.
    fn adds_files(&mut self, path: &Path) -> bool {
        if !path.is_dir() {
            return true;
        }
        let mut adds_files = false;
        for path in existing_paths(slice::from_ref(&path.to_path_buf()), &self.filter) {
            adds_files |= path.is_file();
            self.known.insert(path);
        }
        adds_files
    }

    /// Updates the modification times of the source roots for the package containing
    /// `path`.
    ///
    /// If packages are nested, uses the innermost package.
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

/// A local package with the source roots of its libraries and binaries.
pub struct Package {
    /// The directory containing `Cargo.toml`.
    pub dir: PathBuf,
    /// Source files at the roots of the package's libraries and binaries.
    pub crate_roots: Vec<PathBuf>,
}

/// Updates a file's modification time to now and returns the stored timestamp. Leaves
/// the contents unchanged.
fn touch(path: &Path) -> io::Result<SystemTime> {
    let file = File::options().write(true).open(path)?;
    file.set_modified(SystemTime::now())?;
    file.metadata()?.modified()
}

/// Reads a file's modification time, returning `None` if it cannot be read.
fn modified(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use super::*;

    /// A temporary test package containing `src/main.rs` and `src/en.ftl`.
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
            let fixture = Self { dir };
            fixture.untouch_main();
            fixture
        }

        fn new_files(&self) -> NewFiles {
            self.new_files_in(self.dir.clone())
        }

        /// Creates a tracker for the package in `dir`, using `src/main.rs` as its
        /// source root.
        fn new_files_in(&self, dir: PathBuf) -> NewFiles {
            let filter = PathFilter {
                roots: vec![self.dir.clone()],
                target_dir: None,
                matchers: Vec::new(),
            };
            NewFiles::new(
                slice::from_ref(&self.dir),
                Arc::new(filter),
                vec![Package {
                    dir,
                    crate_roots: vec![self.path("src/main.rs")],
                }],
            )
        }

        /// Resets the modification time of `src/main.rs` so the test can detect a later
        /// touch.
        fn untouch_main(&self) {
            set_modified(&self.path("src/main.rs"), SystemTime::UNIX_EPOCH);
        }

        fn path(&self, file: &str) -> PathBuf {
            self.dir.join(file)
        }

        /// Checks whether the test has touched `src/main.rs`.
        fn main_touched(&self) -> bool {
            modified(&self.path("src/main.rs")).unwrap() > SystemTime::UNIX_EPOCH
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn set_modified(path: &Path, time: SystemTime) {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
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
        fixture.untouch_main();
        assert!(new_files.observe(&ja));
        assert!(!fixture.main_touched());
    }

    #[test]
    fn a_file_moved_out_and_back_in_touches_the_crate_roots() {
        let fixture = Fixture::new("moved");
        let mut new_files = fixture.new_files();
        let en = fixture.path("src/en.ftl");
        let away = fixture.path("en.ftl.away");
        fs::rename(&en, &away).unwrap();
        assert!(new_files.observe(&en));
        assert!(!fixture.main_touched());
        fs::rename(&away, &en).unwrap();
        assert!(new_files.observe(&en));
        assert!(fixture.main_touched());
    }

    #[test]
    fn a_directory_moved_in_touches_the_crate_roots() {
        let fixture = Fixture::new("directory");
        let mut new_files = fixture.new_files();
        let l10n = fixture.path("l10n");
        fs::create_dir(&l10n).unwrap();
        assert!(new_files.observe(&l10n));
        // An empty directory adds no files.
        assert!(!fixture.main_touched());

        let away = fixture.path("l10n.away");
        fs::rename(&l10n, &away).unwrap();
        fs::write(away.join("ja.ftl"), "title = Yokoso\n").unwrap();
        assert!(new_files.observe(&l10n));
        fs::rename(&away, &l10n).unwrap();
        assert!(new_files.observe(&l10n));
        assert!(fixture.main_touched());

        // The files of the directory are known now.
        fixture.untouch_main();
        assert!(new_files.observe(&l10n.join("ja.ftl")));
        assert!(!fixture.main_touched());
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
        set_modified(&main, SystemTime::UNIX_EPOCH + Duration::from_secs(1));
        assert!(new_files.observe(&main));
    }

    #[test]
    fn a_new_file_outside_every_package_touches_nothing() {
        let fixture = Fixture::new("outside");
        let mut new_files = fixture.new_files_in(fixture.path("other"));
        let ja = fixture.path("src/ja.ftl");
        fs::write(&ja, "title = Yokoso\n").unwrap();
        assert!(new_files.observe(&ja));
        assert!(!fixture.main_touched());
    }
}
