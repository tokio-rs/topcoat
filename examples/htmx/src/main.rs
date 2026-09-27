mod increment;

use std::sync::atomic::AtomicU64;

use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
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
        if hx_request(cx) {
            // htmx requests only need the page fragment, not the document.
            (slot)
        } else {
            <!DOCTYPE html>
            <html>
                <head>
                    <script
                        src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
                    ></script>

                    topcoat::dev::script()
                </head>

                // Boost links and forms so htmx can handle navigation.
                <body hx-boost="true">(slot)</body>
            </html>
        }
    })
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>
            "Count: "
            <span id="count">"0"</span>
        </h1>

        // Swaps the returned fragment into #count.
        <button
            hx-post=(href!(crate::increment::increment))
            hx-target="#count"
            hx-swap="innerHTML"
        >
            "Increment"
        </button>
    })
}

struct Counter(AtomicU64);
