mod about;
mod blog;

use topcoat::{
    Result,
    router::{RouterBuilderDiscoverExt, layout, module_router, page},
    view::view,
};

// --- Router ------------------------------------------------------------------

pub fn router() -> topcoat::router::Router {
    // module_router! discovers #[page]/#[layout]/#[route] from the module tree.
    // Converting to RouterBuilder and running discover() picks up PageFn items
    // submitted by mdx_pages! and other inventory-based registrations.
    module_router!().discover().build()
}

// --- Layout ------------------------------------------------------------------

#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <html>
            <head>
                <title>"MDX Blog"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    " | "
                    <a href="/blog">"Blog"</a>
                </nav>
                <hr />
                (slot?)
            </body>
        </html>
    }
}

// --- Home page ---------------------------------------------------------------

#[page]
async fn home() -> Result {
    view! {
        <h1>"Welcome"</h1>
        <p>
            "Check out the "
            <a href="/blog">"blog"</a>
            "."
        </p>
    }
}
