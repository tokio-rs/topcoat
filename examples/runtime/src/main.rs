mod counter;
mod show;
mod sort;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, module_router, page},
    runtime::{RouterBuilderRuntimeExt, link},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .assets(AssetBundle::load().unwrap())
            .discover()
            .runtime()
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

                // Signals, event handlers, and page re-runs need the
                // browser runtime.
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
                </nav>

                <hr>
                <br>

                (slot)
            </body>
        </html>
    })
}
