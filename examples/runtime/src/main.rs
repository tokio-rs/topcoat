mod counter;
mod show;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, module_router, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .assets(AssetBundle::load().unwrap())
            .discover()
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

                // Signals and event handlers need the browser runtime.
                topcoat::runtime::script()
            </head>
            <body>
                <nav>
                    <a href=(href!(counter::page))>"counter"</a>
                    " | "
                    <a href=(href!(show::page))>"show"</a>
                </nav>

                <hr>
                <br>

                (slot)
            </body>
        </html>
    })
}
