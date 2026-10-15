use std::path::{Path, PathBuf};

use clap::Args;
use console::style;

use super::{CACHE_SCOPE, OUT_SUBDIR};

#[derive(Args)]
pub(super) struct CleanArgs {
    /// Asset bundle directory to remove (defaults to every bundle in the
    /// cargo target directory)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

pub(super) async fn run(args: CleanArgs) {
    let Some(target_dir) = crate::common::cargo::Metadata::workspace()
        .await
        .and_then(|metadata| metadata.target_dir())
    else {
        eprintln!(
            "{}",
            style("could not derive cargo target directory; pass --out").red()
        );
        std::process::exit(1);
    };

    let cache_dir = topcoat_core::cache::cache_dir_in(&target_dir, CACHE_SCOPE);
    let mut dirs = match args.out {
        Some(dir) => vec![dir],
        None => bundle_dirs(&target_dir),
    };
    dirs.push(cache_dir);

    for dir in &dirs {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => println!("removed {}", dir.display()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                eprintln!(
                    "{}",
                    style(format!("failed to remove {}: {error}", dir.display())).red()
                );
                std::process::exit(1);
            }
        }
    }
}

/// Finds asset bundle directories under the Cargo target directory. Only directories
/// containing a bundle manifest are included.
fn bundle_dirs(target_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in subdirs(target_dir) {
        candidates.extend(subdirs(&dir));
        candidates.push(dir);
    }
    candidates
        .into_iter()
        .flat_map(|dir| {
            let assets_dir = dir.join(OUT_SUBDIR);
            let mut bundles = vec![assets_dir.clone()];
            bundles.extend(subdirs(&assets_dir));
            bundles
        })
        .filter(|dir| dir.join(topcoat_asset::MANIFEST_NAME).is_file())
        .collect()
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_dirs_finds_scoped_and_legacy_bundles() {
        let target_dir =
            std::env::temp_dir().join(format!("topcoat-cli-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&target_dir);

        let profile_dir = target_dir.join("debug");
        let legacy_bundle = profile_dir.join(OUT_SUBDIR);
        let scoped_bundle = legacy_bundle.join("app");
        let no_manifest = legacy_bundle.join("not-a-bundle");
        std::fs::create_dir_all(&scoped_bundle).unwrap();
        std::fs::create_dir_all(&no_manifest).unwrap();
        std::fs::write(
            legacy_bundle.join(topcoat_asset::MANIFEST_NAME),
            "legacy manifest",
        )
        .unwrap();
        std::fs::write(
            scoped_bundle.join(topcoat_asset::MANIFEST_NAME),
            "scoped manifest",
        )
        .unwrap();

        let mut dirs = bundle_dirs(&target_dir);
        dirs.sort();
        let mut expected = vec![legacy_bundle, scoped_bundle];
        expected.sort();
        assert_eq!(dirs, expected);

        std::fs::remove_dir_all(target_dir).unwrap();
    }
}
