use clap::Args;

use crate::cargo::BuildFlags;

#[derive(Args)]
pub(super) struct ListArgs {
    #[command(flatten)]
    build: BuildFlags,
}

pub(super) async fn run(args: ListArgs) {
    let (_, bytes) = crate::cargo::build_and_read(&args.build.into(), |_, _| {})
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
