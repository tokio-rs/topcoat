mod app;

use topcoat::{
    router::{Slot, layout, page},
    view::{View, view},
};

#[layout]
async fn layout(slot: Slot) -> View {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"hello world"</title>
                [topcoat::dev::script /]
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

#[page]
async fn home_page() -> View {
    view! { "home" }
}

#[page]
async fn about_page() -> View {
    view! { "about" }
}

#[page]
async fn contact_page() -> View {
    view! { "contact" }
}

#[tokio::main]
async fn main() {
    let topcoat_router = app::router();

    let axum_router = axum::Router::new()
        .merge(topcoat_router)
        .route("/axum", axum::routing::get(async || {}));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::serve(listener, axum_router).await.unwrap();
}
