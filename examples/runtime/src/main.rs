mod counter;
mod procedure;
mod record;
mod shard;
mod show;
mod sort;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, module_router, page},
    runtime::{RouterBuilderRuntimeExt, RuntimeConfig, link},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .assets(AssetBundle::load().unwrap())
            .discover()
            .runtime(RuntimeConfig::default())
            .build(),
    )
    .await
    .unwrap();
}

#[page]
async fn page(cx: &Cx) -> Result<()> {
    Err(redirect(href!(counter::page).resolve(cx)).into())
}

#[layout]
async fn layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()

                // Load the browser runtime to enable the interactive examples.
                topcoat::runtime::script()
            </head>
            <body>
                // These links update the page without a full reload.
                // Hovering or focusing a link starts loading its page early.
                <nav>
                    link(href: href!(counter::page), "counter")
                    " | "
                    link(href: href!(show::page), "show")
                    " | "
                    link(href: href!(sort::page), "sort")
                    " | "
                    link(href: href!(procedure::page), "procedure")
                    " | "
                    link(href: href!(shard::page), "shard")
                    " | "
                    link(href: href!(record::page), "record")
                </nav>

                <hr>
                <br>

                (slot)
            </body>
        </html>
    })
}
