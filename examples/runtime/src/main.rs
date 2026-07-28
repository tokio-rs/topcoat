mod counter;
mod show;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{RouterBuilderDiscoverExt, error::redirect, layout, module_router, page},
    view::view,
};

#[tokio::main]
async fn main() {
    // Build the router from the module structure, load the generated assets,
    // and start the server at http://127.0.0.1:3000 by default.
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
    // Redirect the root route to the first runtime example.
    Err(redirect("/counter").into())
}

#[layout]
async fn layout(slot: Result) -> Result {
    // This shared layout wraps both runtime example pages.
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                // Reload the page automatically during development.
                topcoat::dev::script()

                // Load the browser runtime used by signals and event handlers.
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

                // Render the selected runtime example.
                (slot?)
            </body>
        </html>
    }
}
