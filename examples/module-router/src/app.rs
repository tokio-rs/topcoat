mod _marketing;
mod api;
mod docs;

use topcoat::{
    Result,
    router::{href, layout, page},
    view::view,
};

// The `module_router!()` macro call must be placed at the root of your route structure.
// In this case, the `app` module is marked as the root.
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!().build()
}

// A layout in the root app module wraps every page.
#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <html>
            <head>topcoat::dev::script()</head>
            <body>
                // A page as an href target resolves to the path the module
                // tree derives for it, so a moved module updates its links.
                <nav>
                    <a href=(href!(page))>"home"</a>
                    " | "
                    <a href=(href!(about::page))>"about"</a>
                    " | "
                    <a href=(href!(docs::page))>"docs"</a>
                    " | "
                    <a href=(href!(docs::install::page))>"install"</a>
                    " | "
                    <a href=(href!(_marketing::pricing::page))>"pricing"</a>
                </nav>
                <hr>
                (slot?)
            </body>
        </html>
    }
}

// A page in app.rs renders at /.
#[page]
pub async fn page() -> Result {
    view! {
        <h1>"home"</h1>
        <p>"src/app.rs -> /"</p>
    }
}

// The module `about` adds a URL segment `/about`.
mod about {
    use topcoat::{Result, router::page, view::view};

    #[page]
    pub async fn page() -> Result {
        view! {
            <h1>"about"</h1>
            <p>"src/app.rs (mod about) -> /about"</p>
        }
    }
}
