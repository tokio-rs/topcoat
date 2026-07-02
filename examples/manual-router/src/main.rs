use topcoat::{
    Result,
    router::{Router, Slot, layout, page, route},
    view::view,
};

// --- Server -----------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(router()).await.unwrap();
}

// --- Router -----------------------------------------------------------------

// Manual routing means every item has an explicit path and is registered here.
// With the "discover" feature enabled (default), this can instead be:
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

// The root layout wraps every page because every path starts with "/".
#[layout("/")]
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
                </nav>
                <hr>
                (slot.await?)
            </body>
        </html>
    }
}

// This layout wraps /docs and /docs/install.
#[layout("/docs")]
async fn docs_layout(slot: Slot<'_>) -> Result {
    view! {
        <section>
            <p>"docs layout"</p>
            (slot.await?)
        </section>
    }
}

// --- Pages ------------------------------------------------------------------

// Each page declares its own URL path.
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

// Routes are for API requests. This one returns plain text.
#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
