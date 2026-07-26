use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A `compiler-artifact` message from cargo's JSON output: one crate it
/// compiled and the files that produced.
#[derive(Deserialize)]
pub(super) struct Artifact {
    target: Target,
    executable: Option<PathBuf>,
    filenames: Vec<PathBuf>,
}

impl Artifact {
    /// The final linked outputs among this artifact's files: the executable
    /// of a bin target, or the uplifted library outputs of a `cdylib` or
    /// `dylib` target.
    ///
    /// Cargo emits an artifact message for every crate it compiles, but marks
    /// the final outputs itself: `executable` is only set for the requested
    /// bin targets, and only the requested packages' library outputs are
    /// uplifted out of `deps/` into the profile directory. Everything still
    /// in `deps/` is an intermediate dependency and is skipped.
    pub(super) fn final_outputs(&self) -> Vec<PathBuf> {
        if let Some(executable) = &self.executable {
            return vec![executable.clone()];
        }
        if !self.is_dynamic_library() {
            return Vec::new();
        }
        self.filenames
            .iter()
            .filter(|filename| is_final_library(filename))
            .cloned()
            .collect()
    }

    /// Whether the artifact's target is a `cdylib` or `dylib`, as opposed to
    /// a plain `lib` or a proc-macro (whose shared object is uplifted too
    /// when it is a requested target, but is not a scannable application).
    fn is_dynamic_library(&self) -> bool {
        self.target
            .crate_types
            .iter()
            .any(|ty| ty == "cdylib" || ty == "dylib")
    }
}

#[derive(Deserialize)]
struct Target {
    crate_types: Vec<String>,
}

/// Whether `path` names a final dynamic-library output: a linked library by
/// extension (as opposed to an rlib, import library, or dep-info companion in
/// the same `filenames` list) that cargo uplifted out of the `deps/`
/// directory, which it only does for the build's requested targets.
fn is_final_library(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("wasm" | "so" | "dylib" | "dll")
    ) && path
        .parent()
        .and_then(Path::file_name)
        .is_none_or(|dir| dir != "deps")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn artifact(crate_types: &[&str], filenames: &[&str]) -> Artifact {
        serde_json::from_value(json!({
            "target": { "crate_types": crate_types },
            "executable": null,
            "filenames": filenames,
        }))
        .unwrap()
    }

    #[test]
    fn cdylib_output_is_picked_over_its_rlib_companion() {
        let artifact = artifact(
            &["cdylib", "rlib"],
            &["/target/wasm/app.wasm", "/target/wasm/libapp.rlib"],
        );
        assert_eq!(
            artifact.final_outputs(),
            vec![PathBuf::from("/target/wasm/app.wasm")]
        );
    }

    #[test]
    fn dependency_cdylib_outputs_are_excluded() {
        // A dependency that declares `cdylib` produces a scannable output
        // too, but cargo leaves it in `deps/`; only the uplifted output of
        // the crate being built belongs in the bundle.
        let artifact = artifact(&["cdylib", "rlib"], &["/target/deps/libdep.dylib"]);
        assert!(artifact.final_outputs().is_empty());
    }

    #[test]
    fn proc_macro_libraries_are_not_scanned() {
        // A workspace build uplifts the proc-macro members' shared objects
        // right next to the application's output.
        let artifact = artifact(&["proc-macro"], &["/target/libmacros.dylib"]);
        assert!(artifact.final_outputs().is_empty());
    }

    #[test]
    fn plain_library_dependencies_are_not_scanned() {
        let artifact = artifact(&["lib"], &["/target/deps/libdep.rlib"]);
        assert!(artifact.final_outputs().is_empty());
    }

    #[test]
    fn recognizes_final_libraries() {
        assert!(is_final_library(Path::new(
            "/t/wasm32-unknown-unknown/debug/app.wasm"
        )));
        assert!(is_final_library(Path::new("/t/debug/libapp.so")));
        assert!(is_final_library(Path::new("/t/debug/libapp.dylib")));
        assert!(is_final_library(Path::new("/t/debug/app.dll")));
        assert!(!is_final_library(Path::new("/t/debug/deps/app.wasm")));
        assert!(!is_final_library(Path::new("/t/debug/deps/libapp.dylib")));
        assert!(!is_final_library(Path::new("/t/debug/libapp.rlib")));
        assert!(!is_final_library(Path::new("/t/debug/app.dll.lib")));
        assert!(!is_final_library(Path::new("/t/debug/app.d")));
    }
}
