use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, view},
};

// Registered as app context below, so every request shares this counter.
struct PageViews(AtomicU64);

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .discover()
            .app_context(PageViews(AtomicU64::new(0)))
            .build(),
    )
    .await
    .unwrap();
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let views = app_context::<PageViews>(cx);
    let current = views.0.fetch_add(1, Ordering::Relaxed) + 1;

    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"App context"</title>
                topcoat::dev::script()
            </head>
            <body>
                <p>
                    "This page has been viewed "
                    (current)
                    " times."
                </p>
            </body>
        </html>
    })
}
