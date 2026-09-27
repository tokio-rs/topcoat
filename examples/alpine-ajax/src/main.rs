mod increment;

use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    alpine_ajax::ajax_request,
    context::{Cx, app_context},
    router::{Slot, href, layout, module_router, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .app_context(Counter(AtomicU64::new(0)))
            .build(),
    )
    .await
    .unwrap();
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        if ajax_request(cx) {
            // Alpine AJAX requests only need the targeted content, not the document.
            (slot)
        } else {
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
                <body>(slot)</body>
            </html>
        }
    })
}

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    let count = app_context::<Counter>(cx).0.load(Ordering::Relaxed);

    Ok(view! {
        <h1>
            "Count: "
            <span id="count">(count)</span>
        </h1>

        // Alpine AJAX intercepts this form and replaces the element whose
        // id matches `x-target`.
        <form
            method="post"
            action=(href!(crate::increment::increment))
            x-target="count"
        >
            <button type="submit">"Increment"</button>
        </form>
    })
}

struct Counter(AtomicU64);
