mod counter;

use topcoat::{
    Result,
    asset::AssetBundle,
    router::{Slot, layout, module_router, page, redirect},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .assets(AssetBundle::load().unwrap())
            .discover(),
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
                <script type="module" src=(topcoat::runtime::SCRIPT)></script>
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href="/counter">"counter"</a>
                </nav>

                <hr>
                <br>

                (slot.await?)
            </body>
        </html>
    }
}
