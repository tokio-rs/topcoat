use topcoat::{
    Result,
    router::{Router, page},
    view::{component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::new().discover()).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Hello world"</title>
                topcoat::dev::script()
            </head>
            <body>
                hello()
            </body>
        </html>
    }
}

#[component]
async fn hello(name: impl Into<String>) -> Result {
    view! {
        <h1>"Hello, " (name) "!"</h1>
    }
}
