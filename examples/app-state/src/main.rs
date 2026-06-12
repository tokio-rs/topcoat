use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_state},
    router::{Router, page},
    view::view,
};

struct PageViews(AtomicU64);

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::new()
            .discover()
            .app_state(PageViews(AtomicU64::new(0))),
    )
    .await
    .unwrap();
}

// app_state::<T>(cx) borrows the value registered with Router::app_state.
#[page("/")]
async fn home(cx: &Cx) -> Result {
    let views = app_state::<PageViews>(cx);
    let current = views.0.fetch_add(1, Ordering::Relaxed) + 1;

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
            </head>
            <body>
                <p>"This page has been viewed " (current) " times."</p>
            </body>
        </html>
    }
}
