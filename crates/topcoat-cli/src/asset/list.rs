use clap::Args;

use crate::cargo::BuildOpts;

#[derive(Args)]
pub(super) struct ListArgs {
    /// Build and inspect the named binary target
    #[arg(long)]
    bin: Option<String>,
    /// Build and inspect the named package
    #[arg(short, long)]
    package: Option<String>,
}

pub(super) async fn run(args: ListArgs) {
    let opts = BuildOpts {
        bin: args.bin,
        package: args.package,
    };
    let (_, bytes) = crate::cargo::build_and_read(&opts, |_, _| {})
        .await
        .unwrap_or_else(|e| e.print_and_exit());

    for asset in topcoat_asset::RawAsset::find_in_binary(&bytes) {
        match asset.source() {
            topcoat_asset::Source::Path(p) => {
                println!("{}", p.to_str().unwrap_or("<non-utf8 file path>"));
            }
            topcoat_asset::Source::Url(uri) => println!("{uri}"),
        }
    }
}
