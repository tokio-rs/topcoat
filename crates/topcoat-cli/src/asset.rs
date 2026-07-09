mod bundle;
mod clean;
mod list;

use clap::{Args, Subcommand};

pub(crate) use bundle::run_bundle;

const OUT_SUBDIR: &str = "assets";
/// The scope of the asset download cache inside the shared Topcoat cache.
const CACHE_SCOPE: &str = "assets";

#[derive(Args)]
pub struct AssetCommand {
    #[command(subcommand)]
    command: AssetSubcommand,
}

#[derive(Subcommand)]
enum AssetSubcommand {
    /// List all asset paths embedded in the binary produced by cargo
    List(list::ListArgs),
    /// Bundle all assets embedded in the binary into a directory
    Bundle(bundle::BundleArgs),
    /// Delete the asset bundle directory and the asset build cache
    Clean(clean::CleanArgs),
}

impl AssetCommand {
    pub async fn run(self) {
        match self.command {
            AssetSubcommand::List(args) => list::run(args).await,
            AssetSubcommand::Bundle(args) => bundle::run(args).await,
            AssetSubcommand::Clean(args) => clean::run(args).await,
        }
    }
}
