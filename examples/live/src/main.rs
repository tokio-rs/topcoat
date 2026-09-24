mod chat;
mod error_handling;
mod progress;
mod suspense;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, module_router, page},
    runtime::RouterBuilderRuntimeExt,
    view::{View, view},
};

use crate::chat::Chat;

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .assets(AssetBundle::load().unwrap())
            .app_context(Chat::default())
            .discover()
            .runtime()
            .build(),
    )
    .await
    .unwrap();
}

#[page]
async fn page(cx: &Cx) -> Result<()> {
    Err(redirect(href!(suspense::page).resolve(cx)).into())
}

#[layout]
async fn layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Live"</title>

                // Reloads the browser when the dev server rebuilds the app.
                topcoat::dev::script()

                // The chat page's connection and event handlers need the
                // browser runtime.
                topcoat::runtime::script()
            </head>
            <body>
                <nav>
                    <a href=(href!(suspense::page))>"Suspense"</a>
                    " | "
                    <a href=(href!(progress::page))>"Progress"</a>
                    " | "
                    <a href=(href!(error_handling::page))>"Error handling"</a>
                    " | "
                    <a href=(href!(chat::page))>"Chat"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}
