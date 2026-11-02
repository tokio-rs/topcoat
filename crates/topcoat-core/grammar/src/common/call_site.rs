use std::{
    env,
    ffi::OsStr,
    io,
    path::{Component, Path, PathBuf},
};

use proc_macro2::Span;

/// The placeholder for the current module's directory.
const MODULE: &str = "{mod}";

/// The placeholder for the build script's output directory.
const OUT_DIR: &str = "{out_dir}";

/// The placeholders that may start a path, as its whole first component.
pub const BASE_PLACEHOLDERS: [&str; 2] = [MODULE, OUT_DIR];

/// Default directories for binary, test, example and benchmark source roots, relative
/// to the package's manifest directory.
const TARGET_DIRS: [&str; 4] = ["src/bin", "tests", "examples", "benches"];

/// A macro call's location, used to resolve paths written in the call.
pub struct CallSite {
    manifest_dir: PathBuf,
    source_file: Option<PathBuf>,
    out_dir: Option<PathBuf>,
}

impl CallSite {
    /// Creates a call site from a manifest directory, an optional source file on
    /// disk and an optional build script output directory. Resolves relative paths
    /// from the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if it needs the current directory but cannot read it.
    pub fn new(
        manifest_dir: &Path,
        source_file: Option<&Path>,
        out_dir: Option<&Path>,
    ) -> io::Result<Self> {
        Ok(Self {
            manifest_dir: std::path::absolute(manifest_dir)?,
            source_file: source_file.map(std::path::absolute).transpose()?,
            out_dir: out_dir.map(std::path::absolute).transpose()?,
        })
    }

    /// Finds the current macro call's crate, source file and build script output
    /// directory using Cargo and [`Span::call_site`].
    ///
    /// The source file is unknown when the macro does not run in the compiler, such as
    /// in rust-analyzer.
    ///
    /// # Errors
    ///
    /// Returns an error if it needs the current directory but cannot read it.
    pub fn current() -> io::Result<Self> {
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_default();
        let out_dir = env::var_os("OUT_DIR").map(PathBuf::from);
        Self::new(
            &manifest_dir,
            Span::call_site().local_file().as_deref(),
            out_dir.as_deref(),
        )
    }

    /// Returns the directory containing the crate's `Cargo.toml`.
    #[must_use]
    pub fn manifest_dir(&self) -> &Path {
        &self.manifest_dir
    }

    /// Returns the macro call's source file when it exists on disk.
    #[must_use]
    pub fn source_file(&self) -> Option<&Path> {
        self.source_file.as_deref()
    }

    /// Turns a path written in the macro call into an absolute path.
    ///
    /// - A leading `{mod}` is the directory where the current module's child modules
    ///   belong. For both `src/about.rs` and `src/about/mod.rs`, this is `src/about/`.
    /// - A leading `{out_dir}` is the crate's build script output directory.
    /// - Paths starting with `./` or `../` are relative to the source file's directory.
    /// - Absolute paths are used as written.
    /// - Other paths are relative to the directory containing the crate's
    ///   `Cargo.toml`.
    ///
    /// `.` and `..` components are removed from the result.
    ///
    /// # Errors
    ///
    /// Returns an error message if a placeholder is not the whole first path
    /// component, or if the directory a path needs is unknown.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let source_file = || {
            self.source_file.as_deref().ok_or_else(|| {
                format!("cannot resolve `{path}` because the invoking source file is unknown")
            })
        };

        let (first, rest) = path.split_once('/').unwrap_or((path, ""));
        if let Some(placeholder) = BASE_PLACEHOLDERS
            .iter()
            .find(|placeholder| rest.contains(**placeholder) || (first != **placeholder && first.contains(**placeholder)))
        {
            return Err(format!("`{placeholder}` must be the whole first path component"));
        }

        let resolved = match first {
            MODULE => {
                let dir = self
                    .module_dir(source_file()?)
                    .ok_or_else(|| "cannot find the directory of the invoking module".to_owned())?;
                dir.join(rest)
            }
            OUT_DIR => {
                let dir = self.out_dir.as_deref().ok_or_else(|| {
                    format!("cannot resolve `{path}` because the crate has no build script")
                })?;
                dir.join(rest)
            }
            "." | ".." => {
                let dir = source_file()?.parent().unwrap_or(Path::new("/"));
                dir.join(path)
            }
            // Joining an absolute path replaces the manifest directory.
            _ => self.manifest_dir.join(path),
        };
        Ok(normalize(&resolved))
    }

    /// Returns the directory where this module's child modules belong.
    ///
    /// For crate roots and `mod.rs`, this is the source file's directory. For other
    /// files, it is a sibling directory with the file's stem as its name.
    fn module_dir(&self, source_file: &Path) -> Option<PathBuf> {
        let dir = source_file.parent()?;
        let stem = source_file.file_stem()?;
        if stem == "mod" || self.is_crate_root(dir, stem) {
            Some(dir.to_owned())
        } else {
            Some(dir.join(stem))
        }
    }

    /// Checks whether `stem` in `dir` is a target's source root under Cargo's default
    /// directory layout.
    fn is_crate_root(&self, dir: &Path, stem: &OsStr) -> bool {
        let is_target_dir = |dir: &Path| {
            TARGET_DIRS
                .iter()
                .any(|target_dir| dir == self.manifest_dir.join(target_dir))
        };
        is_target_dir(dir)
            || ((stem == "lib" || stem == "main") && dir == self.manifest_dir.join("src"))
            || (stem == "main" && dir.parent().is_some_and(is_target_dir))
    }
}

/// Removes `.` and `..` components from an absolute path without reading the file
/// system.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolves `path` from `source_file` in a crate at `/app` without a build script.
    fn resolve(source_file: Option<&str>, path: &str) -> Result<PathBuf, String> {
        let manifest_dir = Path::new("/app");
        let source_file = source_file.map(|file| manifest_dir.join(file));
        let call_site = CallSite::new(manifest_dir, source_file.as_deref(), None).unwrap();
        call_site.resolve(path)
    }

    fn absolute(path: &str) -> Result<PathBuf, String> {
        Ok(std::path::absolute(path).unwrap())
    }

    #[test]
    fn resolves_plain_paths_from_the_manifest_directory() {
        assert_eq!(
            resolve(Some("src/pages/about.rs"), "assets/a.png"),
            absolute("/app/assets/a.png")
        );
    }

    #[test]
    fn resolves_dot_paths_from_the_source_file_directory() {
        let about = Some("src/pages/about.rs");
        assert_eq!(resolve(about, "./a.png"), absolute("/app/src/pages/a.png"));
        assert_eq!(resolve(about, "../a.png"), absolute("/app/src/a.png"));
        assert_eq!(
            resolve(about, "./img/../a.png"),
            absolute("/app/src/pages/a.png")
        );
    }

    #[test]
    fn keeps_absolute_paths() {
        assert_eq!(resolve(None, "/srv/a.png"), absolute("/srv/a.png"));
    }

    #[test]
    fn resolves_the_module_directory_of_a_module_file() {
        let module = |file| resolve(Some(file), "{mod}/a.png");
        assert_eq!(module("src/pages/about.rs"), absolute("/app/src/pages/about/a.png"));
        assert_eq!(module("src/pages/mod.rs"), absolute("/app/src/pages/a.png"));
        // A module that happens to be named `main` is not a crate root.
        assert_eq!(module("src/pages/main.rs"), absolute("/app/src/pages/main/a.png"));
    }

    #[test]
    fn resolves_the_module_directory_of_a_crate_root() {
        let module = |file| resolve(Some(file), "{mod}/a.png");
        assert_eq!(module("src/lib.rs"), absolute("/app/src/a.png"));
        assert_eq!(module("src/main.rs"), absolute("/app/src/a.png"));
        assert_eq!(module("src/bin/cli.rs"), absolute("/app/src/bin/a.png"));
        assert_eq!(module("src/bin/tool/main.rs"), absolute("/app/src/bin/tool/a.png"));
        assert_eq!(module("tests/smoke.rs"), absolute("/app/tests/a.png"));
    }

    #[test]
    fn resolves_the_build_script_output_directory() {
        let call_site =
            CallSite::new(Path::new("/app"), None, Some(Path::new("/target/out"))).unwrap();
        assert_eq!(
            call_site.resolve("{out_dir}/style.css"),
            absolute("/target/out/style.css")
        );
        assert!(resolve(None, "{out_dir}/style.css").is_err());
    }

    #[test]
    fn rejects_a_misplaced_placeholder() {
        let lib = Some("src/lib.rs");
        assert!(resolve(lib, "{mod}x/a.png").is_err());
        assert!(resolve(lib, "./{mod}/a.png").is_err());
        assert!(resolve(lib, "assets/{mod}/a.png").is_err());
        assert!(resolve(lib, "{mod}/{mod}/a.png").is_err());
        assert!(resolve(lib, "{mod}/{out_dir}/a.png").is_err());
    }

    #[test]
    fn rejects_source_relative_paths_without_a_source_file() {
        assert!(resolve(None, "./a.png").is_err());
        assert!(resolve(None, "{mod}/a.png").is_err());
    }
}
