use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

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

// app_context::<T>(cx) borrows the value registered with Router::app_context.
#[page("/")]
async fn home(cx: &Cx) -> Result {
    let views = app_context::<PageViews>(cx);
    let current = views.0.fetch_add(1, Ordering::Relaxed) + 1;

    view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
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
