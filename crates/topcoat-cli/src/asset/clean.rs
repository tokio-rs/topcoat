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

/// Every asset bundle in the target directory. Bundles are written next to
/// the executable they were scanned from, so they sit at
/// `<target>/<profile>/assets` and, for cross builds,
/// `<target>/<triple>/<profile>/assets`. Only directories holding a manifest
/// are returned, so an unrelated `assets` directory in the target tree is
/// left alone.
fn bundle_dirs(target_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in subdirs(target_dir) {
        candidates.extend(subdirs(&dir));
        candidates.push(dir);
    }
    candidates
        .into_iter()
        .map(|dir| dir.join(OUT_SUBDIR))
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
