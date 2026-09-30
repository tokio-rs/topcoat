use std::{
    error::Error,
    path::{Path, PathBuf},
};

use clap::Args;
use console::style;

use super::{CACHE_SCOPE, OUT_SUBDIR};
use crate::common::cargo::{BuildFlags, BuildOpts, Metadata};

#[derive(Args)]
pub(super) struct BundleArgs {
    #[command(flatten)]
    build: BuildFlags,
    /// Output directory for the bundle (defaults to an `assets` directory
    /// next to the built executable)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

pub(super) async fn run(args: BundleArgs) {
    let (exe, bytes) = BuildOpts::from(args.build)
        .build_and_read(|_, _| {})
        .await
        .unwrap_or_else(|e| e.print_and_exit());

    let out_dir = match run_bundle(&exe, &bytes, args.out).await {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "{}",
                style(format!("failed to bundle assets: {error}")).red()
            );
            std::process::exit(1);
        }
    };

    println!("bundled assets into {}", out_dir.display());
}

/// Bundles assets declared in the executable.
///
/// `bytes` must contain the file at `exe`. Writes the bundle to `out_override`, or to
/// `assets` beside the executable by default. Keeping bundles beside their executable
/// prevents builds with different asset IDs from sharing the wrong bundle.
pub(crate) async fn run_bundle(
    exe: &Path,
    bytes: &[u8],
    out_override: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let target_dir = Metadata::workspace()
        .await
        .and_then(|metadata| metadata.target_dir())
        .ok_or("could not derive cargo target directory")?;
    let out_dir = match out_override {
        Some(dir) => dir,
        None => exe
            .parent()
            .ok_or("built executable has no parent directory")?
            .join(OUT_SUBDIR),
    };
    let cache_dir = topcoat_core::cache::cache_dir_in(&target_dir, CACHE_SCOPE);

    // `bundle` blocks on filesystem and network I/O, so run it off the runtime.
    let bytes = bytes.to_vec();
    let bundle_dir = out_dir.clone();
    // A unified target directory (`CARGO_TARGET_DIR`) hosts several apps'
    // bundles, so each app isolates into its own `assets/<exe-stem>/`
    // subdirectory and never collects another app's assets. A
    // project-local target directory keeps the flat layout.
    let has_shared_dir = std::env::var_os("CARGO_TARGET_DIR").is_some();
    let manifest_stem = exe
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or("built executable has no file stem")?
        .to_owned();
    tokio::task::spawn_blocking(move || {
        let config = topcoat_asset::BundlerConfig::new().cache_dir(cache_dir);
        let bundler = topcoat_asset::Bundler::new(&config);
        if has_shared_dir {
            bundler.bundle_as(&bytes, &bundle_dir, &manifest_stem)
        } else {
            bundler.bundle(&bytes, &bundle_dir)
        }
    })
    .await??;
    Ok(out_dir)
}
