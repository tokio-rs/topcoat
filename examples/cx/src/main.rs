use topcoat::{
    Result,
    context::Cx,
    router::{Router, headers, page, uri},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::new().discover()).await.unwrap();
}

// Small functions can take cx and read request data directly.
fn current_path(cx: &Cx) -> &str {
    uri(cx).path()
}

fn user_agent(cx: &Cx) -> &str {
    headers(cx)
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Cx"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Cx functions"</h1>
                <p>"path: " (current_path(cx))</p>
                <p>"user agent: " (user_agent(cx))</p>
            </body>
        </html>
    }
}
