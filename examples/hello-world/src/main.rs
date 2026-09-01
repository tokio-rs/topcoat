use std::time::Duration;

use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, emit, live, view},
};

#[tokio::main]
async fn main() {
    // `discover` picks up every page, layout, and route declared in the crate.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Hello world"</title>
                // Reloads the browser when the dev server rebuilds the app.
                topcoat::dev::script()
            </head>
            <body>
                (live! {
                    for count in 0..100 {
                        emit! {
                            "Loading... "
                            (count)
                            "%"
                        }?;
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    emit! { "Hello world!" }
                })
            </body>
        </html>
    })
}
