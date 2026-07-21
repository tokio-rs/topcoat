use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    alpine_ajax::ajax_request,
    context::{Cx, app_context},
    router::{
        IntoResponse, Response, Router, RouterBuilderDiscoverExt, Slot, layout, page, route,
        see_other,
    },
    view::view,
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
    // Alpine AJAX automatically merges just the targeted content.
    if ajax_request(cx) {
        return slot.await;
    }

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                // `defer` matters here: without it, Alpine can start
                // initializing before `<body>` exists and silently skip
                // binding directives on the page's first render.
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
            <body>(slot.await?)</body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let count = app_context::<Counter>(cx).0.load(Ordering::Relaxed);

    view! {
        <h1>
            "Count: "
            <span id="count">(count)</span>
        </h1>
        <form method="post" action="/increment" x-target="count">
            <button type="submit">"Increment"</button>
        </form>
    }
}

// A shared counter, registered as app context.
struct Counter(AtomicU64);

// Bumps the counter. For an Alpine AJAX request, returns just the `<span>`
// wrapping the new value, which Alpine AJAX merges into whichever elements
// the request targeted (here, `#count`, per the form's `x-target="count"`).
//
// The form is a plain HTML `<form method="post">`, so this also has to work
// with JavaScript disabled: the browser then submits a full-page request,
// which isn't an Alpine AJAX request, so this falls back to the
// Post/Redirect/Get pattern, sending the browser back to `/` to see the
// updated count.
#[route(POST "/increment")]
async fn increment(cx: &Cx) -> Result<Response> {
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;

    if ajax_request(cx) {
        return view! { <span id="count">(count)</span> }?.into_response(cx);
    }

    see_other("/").into_response(cx)
}
