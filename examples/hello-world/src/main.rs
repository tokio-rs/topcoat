use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, component, view},
};

#[tokio::main]
async fn main() {
    // `discover` registers every page, layout, and route that declares its own
    // path, so none of them has to be listed here.
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
            <body>hello(name: "World")</body>
        </html>
    })
}

#[component]
async fn hello(name: &str) -> Result<impl View> {
    Ok(view! {
        <h1>
            "Hello, "
            (name)
            "!"
        </h1>
    })
}
