mod counter;
mod show;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{RouterBuilderDiscoverExt, Slot, layout, module_router, page, redirect},
    view::view,
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
async fn home() -> Result {
    Err(redirect("/counter").into())
}

#[layout]
async fn layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
                topcoat::runtime::script()
            </head>
            <body>
                <nav>
                    <a href="/counter">"counter"</a>
                    " | "
                    <a href="/show">"show"</a>
                </nav>

                <hr>
                <br>

                (slot.await?)
            </body>
        </html>
    }
}
