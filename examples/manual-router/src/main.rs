use topcoat::{
    Result,
    router::{Router, layout, page, route},
    view::{ViewHandle, view},
};

// --- Server -----------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(router()).await.unwrap();
}

// --- Router -----------------------------------------------------------------

// Without `.discover()`, every layout, page, and route is registered by hand.
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

// Wraps every page, because every page path starts with `/`.
#[layout("/")]
async fn root_layout(slot: ViewHandle<'_>) -> Result {
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

                // The page, or the nested layout wrapping it.
                (slot)
            </body>
        </html>
    }
}

// Wraps `/docs` and every page below it.
#[layout("/docs")]
async fn docs_layout(slot: ViewHandle<'_>) -> Result {
    view! {
        <section>
            <p>"docs layout"</p>
            (slot)
        </section>
    }
}

// --- Pages ------------------------------------------------------------------

// A page declares its own path, but still has to be registered in `router`.
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

// Routes return responses other than pages, and skip the layouts.
#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
