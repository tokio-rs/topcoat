mod _marketing;
mod api;
mod docs;

use topcoat::{
    Result,
    router::{Slot, layout, page},
    view::view,
};

// The `module_router!()` macro call must be placed at the root of your route structure.
// In this case, the `app` module is marked as the root.
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!().build()
}

// A layout in the root app module wraps every page.
#[layout]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <html>
            <head>topcoat::dev::script()</head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    " | "
                    <a href="/about">"about"</a>
                    " | "
                    <a href="/docs">"docs"</a>
                    " | "
                    <a href="/docs/install">"install"</a>
                    " | "
                    <a href="/pricing">"pricing"</a>
                </nav>
                <hr>
                (slot.await?)
            </body>
        </html>
    }
}

// A page in app.rs renders at /.
#[page]
async fn home() -> Result {
    view! {
        <h1>"home"</h1>
        <p>"src/app.rs -> /"</p>
    }
}

// The module `about` adds a URL segment `/about`.
mod about {
    use topcoat::{Result, router::page, view::view};

    #[page]
    async fn about() -> Result {
        view! {
            <h1>"about"</h1>
            <p>"src/app.rs (mod about) -> /about"</p>
        }
    }
}
