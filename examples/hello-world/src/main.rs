use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{component, view},
};

#[tokio::main]
async fn main() {
    // Discover the routes declared in this example and start the HTTP server.
    // By default, the app is available at http://127.0.0.1:3000.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    // This page is rendered when a browser requests the root route (`/`).
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Hello world"</title>

                // Enables automatic browser reload when using the Topcoat development server.
                topcoat::dev::script()
            </head>
            <body>hello(name: "World")</body>
        </html>
    }
}

#[component]
async fn hello(name: &str) -> Result {
    // Components can accept arguments and render reusable HTML.
    view! {
        <h1>
            "Hello, "
            (name)
            "!"
        </h1>
    }
}
