use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    htmx::{HxResponseTrigger, hx_request},
    router::{Router, RouterBuilderDiscoverExt, Slot, layout, page, route},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .discover()
            .app_context(Counter(AtomicU64::new(0)))
            .build(),
    )
    .await
    .unwrap();
}

#[layout("/")]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result {
    // For client-side navigations we don't need to return the full HTML shell again.
    // htmx automatically swaps out just the body.
    if hx_request(cx) {
        return slot.await;
    }

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script
                    src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js">
                </script>
                topcoat::dev::script()
            </head>
            <body hx-boost="true">(slot.await?)</body>
        </html>
    }
}

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>
            "Count: "
            <span id="count">"0"</span>
        </h1>
        <button hx-post="/increment" hx-target="#count" hx-swap="innerHTML">"Increment"
        </button>
    }
}

// A shared counter, registered as app context.
struct Counter(AtomicU64);

// Bumps the counter and returns just the `<span>` wrapping the new value.
//
// `HxResponseTrigger` adds an `HX-Trigger` response header that fires
// a `counted` event on the client.
#[route(POST "/increment")]
async fn increment(cx: &Cx) -> Result<(HxResponseTrigger, View)> {
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;

    let fragment = view! { <span id="count">(count)</span> }?;
    Ok((HxResponseTrigger::receive(["counted"]), fragment))
}
