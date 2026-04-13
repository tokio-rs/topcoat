// mod app;
// mod components;

use topcoat::{axum::routing::get, view};

#[tokio::main]
async fn main() {
    // let router = app::router();

    let router = topcoat::axum::Router::new().route(
        "/",
        get(async || {
            view! {
                <!DOCTYPE html>
                <html>
                    <head>
                        <title>"hello world"</title>
                    </head>
                    <body id="test">
                        <form
                            // action=async || {
                            //     // runs on server
                            //     println!("{}, {}", email, password);
                            // }
                        >
                            <input name="email">
                            <input name="password">
                        </form>
                    </body>
                </html>
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::axum::serve(listener, router).await.unwrap();
}
