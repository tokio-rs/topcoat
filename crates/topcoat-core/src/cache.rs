//! The shared Topcoat cache for files that build scripts download.
//!
//! The cache lives at `topcoat/cache/<scope>` inside the Cargo target
//! directory. Every package of a workspace shares it, and it survives
//! rebuilds, though not `cargo clean`.

use std::{
    env,
    path::{Path, PathBuf},
};

/// Returns the Topcoat cache directory for `scope` inside `target_dir`, which
/// is `<target_dir>/topcoat/cache/<scope>`.
#[must_use]
pub fn cache_dir_in(target_dir: impl AsRef<Path>, scope: &str) -> PathBuf {
    target_dir
        .as_ref()
        .join("topcoat")
        .join("cache")
        .join(scope)
}

/// Returns the Topcoat cache directory for `scope`, for use from a build
/// script.
///
/// The Cargo target directory is found from `OUT_DIR`. For cross builds this
/// is the target-triple subdirectory. Returns `None` when `OUT_DIR` is unset
/// or does not follow Cargo's `build/<package>-<hash>/out` layout.
#[must_use]
pub fn cache_dir(scope: &str) -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?);
    Some(cache_dir_in(target_dir(&out_dir)?, scope))
}

/// The Cargo target directory (or its target-triple subdirectory) containing
/// `out_dir`, which Cargo lays out as
/// `<target>[/<triple>]/<profile>/build/<package>-<hash>/out`.
fn target_dir(out_dir: &Path) -> Option<&Path> {
    let build = out_dir.parent()?.parent()?;
    if build.file_name()? != "build" {
        return None;
    }
    build.parent()?.parent()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_dirs_resolve_from_out_dirs() {
        let dir = target_dir(Path::new("/repo/target/debug/build/app-0123abc/out"));
        assert_eq!(dir, Some(Path::new("/repo/target")));
    }

    #[test]
    fn cross_builds_resolve_to_the_triple_subdirectory() {
        let out_dir = "/repo/target/aarch64-unknown-linux-gnu/release/build/app-0123abc/out";
        let dir = target_dir(Path::new(out_dir));
        assert_eq!(
            dir,
            Some(Path::new("/repo/target/aarch64-unknown-linux-gnu"))
        );
    }

    #[test]
    fn foreign_layouts_do_not_resolve() {
        assert_eq!(target_dir(Path::new("/sandbox/outputs/out")), None);
        assert_eq!(target_dir(Path::new("/")), None);
    }

    #[test]
    fn cache_dirs_follow_the_shared_convention() {
        assert_eq!(
            cache_dir_in("/repo/target", "assets"),
            PathBuf::from("/repo/target/topcoat/cache/assets")
        );
    }
}
