use std::error::Error;
use std::path::PathBuf;

use clap::Args;
use console::style;

use crate::cargo::BuildFlags;

use super::{CACHE_SUBDIR, OUT_SUBDIR};

#[derive(Args)]
pub(super) struct BundleArgs {
    #[command(flatten)]
    build: BuildFlags,
    /// Output directory for the bundle (defaults to <cargo-target>/assets)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

pub(super) async fn run(args: BundleArgs) {
    let (_, bytes) = crate::cargo::build_and_read(&args.build.into(), |_, _| {})
        .await
        .unwrap_or_else(|e| e.print_and_exit());

    let out_dir = match run_bundle(&bytes, args.out).await {
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

pub(crate) async fn run_bundle(
    bytes: &[u8],
    out_override: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let target_dir = crate::cargo::target_dir()
        .await
        .ok_or("could not derive cargo target directory")?;
    let out_dir = out_override.unwrap_or_else(|| target_dir.join(OUT_SUBDIR));
    let cache_dir = target_dir.join(CACHE_SUBDIR);

    // `bundle` blocks on filesystem and network I/O, so run it off the runtime.
    let bytes = bytes.to_vec();
    let bundle_dir = out_dir.clone();
    tokio::task::spawn_blocking(move || {
        topcoat_asset::Bundler::new(cache_dir).bundle(&bytes, &bundle_dir)
    })
    .await??;
    Ok(out_dir)
}
