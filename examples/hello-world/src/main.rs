use topcoat::{View, axum::routing::get, layout, page, router::layout::Slot, view};

#[layout]
async fn layout(slot: Slot) -> View {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"hello world"</title>
            </head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    <span>" | "</span>
                    <a href="/about">"about"</a>
                    <span>" | "</span>
                    <a href="/contact">"contact"</a>
                </nav>
                <hr>

                "current page: "
                (slot.await)
            </body>
        </html>
    }
}

#[page("/")]
async fn home_page() -> View {
    view! { "home" }
}

#[page("/about")]
async fn about_page() -> View {
    view! { "about" }
}

#[page("/contact")]
async fn contact_page() -> View {
    view! { "contact" }
}

#[tokio::main]
async fn main() {
    let topcoat_router = topcoat::router::Router::new()
        .layout(layout)
        .page(home_page)
        .page(about_page)
        .page(contact_page);

    let axum_router = axum::Router::new()
        .merge(topcoat_router)
        .route("/axum", get(async || {}));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_router).await.unwrap();
}
