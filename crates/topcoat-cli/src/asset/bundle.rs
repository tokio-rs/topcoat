use std::{
    error::Error,
    path::{Path, PathBuf},
};

use clap::Args;
use console::style;

use super::CACHE_SCOPE;
use crate::common::cargo::{BuildFlags, BuildOpts, Metadata};

#[derive(Args)]
pub(super) struct BundleArgs {
    #[command(flatten)]
    build: BuildFlags,
    /// Output directory for the bundle (defaults to an artifact-scoped
    /// directory under `assets` next to the built artifact)
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

/// Bundles assets declared in the built artifact.
///
/// `bytes` must contain the artifact at `exe`. Writes the bundle to `out_override`, or to
/// an artifact-scoped directory under `assets` beside the built output by default.
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
        None => topcoat_asset::AssetBundle::dir_for_artifact(exe)?,
    };
    let cache_dir = topcoat_core::cache::cache_dir_in(&target_dir, CACHE_SCOPE);

    // `bundle` blocks on filesystem and network I/O, so run it off the runtime.
    let bytes = bytes.to_vec();
    let bundle_dir = out_dir.clone();
    tokio::task::spawn_blocking(move || {
        let config = topcoat_asset::BundlerConfig::new().cache_dir(cache_dir);
        topcoat_asset::Bundler::new(&config).bundle(&bytes, &bundle_dir)
    })
    .await??;
    Ok(out_dir)
}
