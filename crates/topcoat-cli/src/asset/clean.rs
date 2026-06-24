use std::path::PathBuf;

use clap::Args;
use console::style;

use super::{CACHE_SUBDIR, OUT_SUBDIR};

#[derive(Args)]
pub(super) struct CleanArgs {
    /// Asset bundle directory to remove (defaults to <cargo-target>/assets)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

pub(super) async fn run(args: CleanArgs) {
    let Some(target_dir) = crate::cargo::target_dir().await else {
        eprintln!(
            "{}",
            style("could not derive cargo target directory; pass --out").red()
        );
        std::process::exit(1);
    };

    let out_dir = args.out.unwrap_or_else(|| target_dir.join(OUT_SUBDIR));
    let cache_dir = target_dir.join(CACHE_SUBDIR);

    for dir in [&out_dir, &cache_dir] {
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
