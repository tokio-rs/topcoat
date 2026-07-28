use topcoat::{
    Result,
    router::{Router, layout, page, route},
    view::view,
};

// --- Server -----------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Build the router manually and start the HTTP server.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(router()).await.unwrap();
}

// --- Router -----------------------------------------------------------------

// Manual routing means every layout, page, and route is explicitly registered.
//
// With automatic discovery enabled, this could instead use:
//
// fn router() -> Router {
//     Router::builder().discover().build()
// }
fn router() -> Router {
    Router::builder()
        .layout(root_layout)
        .layout(docs_layout)
        .page(home)
        .page(about)
        .page(docs)
        .page(install)
        .route(health)
        .build()
}

// --- Layouts ----------------------------------------------------------------

// The root layout wraps every page because every registered page path starts
// with `/`.
#[layout("/")]
async fn root_layout(slot: Result) -> Result {
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
                </nav>

                <hr>

                // Render the page or nested layout inside this layout.
                (slot?)
            </body>
        </html>
    }
}

// This nested layout wraps `/docs` and every page below that path.
#[layout("/docs")]
async fn docs_layout(slot: Result) -> Result {
    view! {
        <section>
            <p>"docs layout"</p>
            (slot?)
        </section>
    }
}

// --- Pages ------------------------------------------------------------------

// Each page declares its own URL path, but must also be registered in router().
#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"home"</h1>
        <p>"registered with .page(home)"</p>
    }
}

#[page("/about")]
async fn about() -> Result {
    view! {
        <h1>"about"</h1>
        <p>"#[page(\"/about\")]"</p>
    }
}

#[page("/docs")]
async fn docs() -> Result {
    view! {
        <h1>"docs"</h1>
        <p>"wrapped by #[layout(\"/docs\")]"</p>
    }
}

#[page("/docs/install")]
async fn install() -> Result {
    view! {
        <h1>"install"</h1>
        <p>"also wrapped by #[layout(\"/docs\")]"</p>
    }
}

// --- Routes -----------------------------------------------------------------

// Routes can return non-page responses. This API endpoint returns plain text.
#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
