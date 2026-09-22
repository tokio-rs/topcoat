mod error_handling;
mod progress;
mod suspense;

use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, module_router, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().discover().build())
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
            </head>
            <body>
                <nav>
                    <a href=(href!(suspense::page))>"Suspense"</a>
                    " | "
                    <a href=(href!(progress::page))>"Progress"</a>
                    " | "
                    <a href=(href!(error_handling::page))>"Error handling"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}
