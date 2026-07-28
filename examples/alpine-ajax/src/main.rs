use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    alpine_ajax::ajax_request,
    context::{Cx, app_context},
    router::{
        IntoResponse, Response, Router, RouterBuilderDiscoverExt, error::see_other, layout, page,
        route,
    },
    view::view,
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
    // Alpine AJAX requests only need the targeted content.
    // Normal browser requests receive the complete HTML document.
    if ajax_request(cx) {
        return slot;
    }

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                // `defer` ensures that Alpine initializes after the page body
                // has been parsed.
                <script
                    defer=""
                    src="https://cdn.jsdelivr.net/npm/@imacrayon/alpine-ajax@0.12.4/dist/cdn.min.js"
                ></script>
                <script
                    defer=""
                    src="https://cdn.jsdelivr.net/npm/alpinejs@3.15.0/dist/cdn.min.js"
                ></script>

                topcoat::dev::script()
            </head>
            <body>(slot?)</body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    // Read the current value from the application context.
    let count = app_context::<Counter>(cx).0.load(Ordering::Relaxed);

    view! {
        <h1>
            "Count: "
            <span id="count">(count)</span>
        </h1>

        // Alpine AJAX intercepts this form and replaces the element whose
        // id matches `x-target`.
        <form method="post" action="/increment" x-target="count">
            <button type="submit">"Increment"</button>
        </form>
    }
}

// The counter is shared across every request handled by this process.
struct Counter(AtomicU64);

#[route(POST "/increment")]
async fn increment(cx: &Cx) -> Result<Response> {
    // Increment the shared value and obtain the new count.
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;

    // For an Alpine AJAX request, return only the targeted element.
    if ajax_request(cx) {
        return view! { <span id="count">(count)</span> }?.into_response(cx);
    }

    // Without JavaScript, use Post/Redirect/Get and render the complete page.
    see_other("/").into_response(cx)
}
