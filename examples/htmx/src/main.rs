use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    htmx::{HxResponseTrigger, hx_request},
    router::{Router, RouterBuilderDiscoverExt, href, layout, page, route},
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
async fn root(cx: &Cx, slot: Result) -> Result {
    // htmx requests only need the page fragment, not the document.
    if hx_request(cx) {
        return slot;
    }

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script
                    src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
                ></script>

                topcoat::dev::script()
            </head>

            // Boost links and forms so htmx can handle navigation.
            <body hx-boost="true">(slot?)</body>
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

        // Swaps the returned fragment into #count.
        <button hx-post=(href(increment, ())) hx-target="#count" hx-swap="innerHTML">
            "Increment"
        </button>
    }
}

struct Counter(AtomicU64);

#[route(POST "/increment")]
async fn increment(cx: &Cx) -> Result<(HxResponseTrigger, View)> {
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;
    let fragment = view! { <span id="count">(count)</span> }?;

    // The trigger becomes an `HX-Trigger: counted` response header, which
    // fires a `counted` event in the browser.
    Ok((HxResponseTrigger::receive(["counted"]), fragment))
}
