use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, app_context},
    htmx::{HxResponseTrigger, hx_request},
    router::{Router, RouterBuilderDiscoverExt, layout, page, route},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    // Discover the routes, register the shared counter, and start the server.
    // By default, the application is available at http://127.0.0.1:3000.
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
    // htmx requests only need the page fragment.
    // Regular browser requests receive the complete HTML document.
    if hx_request(cx) {
        return slot;
    }

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                // Load htmx from a CDN.
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

        // Send POST /increment and replace only the contents of #count
        // with the fragment returned by the server.
        <button hx-post="/increment" hx-target="#count" hx-swap="innerHTML">
            "Increment"
        </button>
    }
}

// Counter shared by all requests handled by this server process.
struct Counter(AtomicU64);

#[route(POST "/increment")]
async fn increment(cx: &Cx) -> Result<(HxResponseTrigger, View)> {
    // Increment the shared counter and obtain the new value.
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;

    // Return the updated HTML fragment.
    let fragment = view! { <span id="count">(count)</span> }?;

    // Add `HX-Trigger: counted` so the browser also receives
    // a custom htmx event named `counted`.
    Ok((HxResponseTrigger::receive(["counted"]), fragment))
}
