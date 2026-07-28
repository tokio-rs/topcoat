use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// Application state shared across all requests.
// AtomicU64 allows multiple requests to update the counter safely.
struct PageViews(AtomicU64);

#[tokio::main]
async fn main() {
    // Discover the routes and register PageViews as application context.
    // By default, the server is available at http://127.0.0.1:3000.
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
async fn home(cx: &Cx) -> Result {
    // Borrow the PageViews value registered when the router was created.
    let views = app_context::<PageViews>(cx);

    // Increment the shared counter and calculate the value to display.
    let current = views.0.fetch_add(1, Ordering::Relaxed) + 1;

    view! {
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
    }
}
