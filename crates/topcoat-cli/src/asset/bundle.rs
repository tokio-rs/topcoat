use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use clap::Args;
use console::style;
use topcoat_asset::FetchEvent;

use crate::cargo::BuildFlags;
use crate::dev::spinner::Spinner;

use super::{CACHE_SCOPE, OUT_SUBDIR};

#[derive(Args)]
pub(super) struct BundleArgs {
    #[command(flatten)]
    build: BuildFlags,
    /// Output directory for the bundle (defaults to <cargo-target>/assets)
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Print nothing but the bundle directory and errors
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Also print remote assets served from the download cache
    #[arg(short, long)]
    verbose: bool,
}

pub(super) async fn run(args: BundleArgs) {
    let spinner = Spinner::new("building");
    let progress = spinner.bar();
    let build = crate::cargo::build_and_read(&args.build.into(), move |current, total| {
        progress.set_message(format!("building ({current}/{total})"));
    })
    .await;
    drop(spinner);
    let (_, bytes) = build.unwrap_or_else(|e| e.print_and_exit());

    let quiet = args.quiet;
    let verbose = args.verbose;
    let cache_hits = Arc::new(AtomicUsize::new(0));
    let hits = Arc::clone(&cache_hits);
    let result = run_bundle(&bytes, args.out, move |event| match event {
        FetchEvent::Downloaded { uri, elapsed } => {
            if !quiet {
                println!(
                    "{} {uri} ({})",
                    style("downloaded").green(),
                    format_elapsed(elapsed)
                );
            }
        }
        FetchEvent::CacheHit { uri } => {
            if verbose {
                println!("{} {uri}", style("cached").dim());
            }
            hits.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    })
    .await;

    let out_dir = match result {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "{}",
                style(format!("failed to bundle assets: {error}")).red()
            );
            std::process::exit(1);
        }
    };

    let cache_hits = cache_hits.load(Ordering::Relaxed);
    if cache_hits > 0 && !args.quiet && !args.verbose {
        println!(
            "{}",
            style(format!("{cache_hits} remote assets already cached")).dim()
        );
    }
    println!("bundled assets into {}", out_dir.display());
}

fn format_elapsed(elapsed: Duration) -> String {
    if elapsed.as_secs() >= 1 {
        format!("{:.1}s", elapsed.as_secs_f64())
    } else {
        format!("{}ms", elapsed.as_millis())
    }
}

pub(crate) async fn run_bundle(
    bytes: &[u8],
    out_override: Option<PathBuf>,
    on_fetch: impl Fn(FetchEvent<'_>) + Send + Sync + 'static,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let target_dir = crate::cargo::target_dir()
        .await
        .ok_or("could not derive cargo target directory")?;
    let out_dir = out_override.unwrap_or_else(|| target_dir.join(OUT_SUBDIR));
    let cache_dir = topcoat_core::cache::cache_dir_in(&target_dir, CACHE_SCOPE);

    // `bundle` blocks on filesystem and network I/O, so run it off the runtime.
    let bytes = bytes.to_vec();
    let bundle_dir = out_dir.clone();
    tokio::task::spawn_blocking(move || {
        topcoat_asset::Bundler::new(cache_dir)
            .on_fetch(on_fetch)
            .bundle(&bytes, &bundle_dir)
    })
    .await??;
    Ok(out_dir)
}
