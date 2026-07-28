use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, headers, page, uri},
    view::view,
};

#[tokio::main]
async fn main() {
    // Discover the routes defined in this example and start the server.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

// Read the path from the current HTTP request.
fn current_path(cx: &Cx) -> &str {
    uri(cx).path()
}

// Read the User-Agent header from the current HTTP request.
// Return "unknown" if the header is missing or cannot be converted to text.
fn user_agent(cx: &Cx) -> &str {
    headers(cx)
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    // `Cx` contains information about the current request and can be
    // passed to helper functions while rendering the response.
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Cx"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Cx functions"</h1>

                // Display the path requested by the client.
                <p>
                    "path: "
                    (current_path(cx))
                </p>

                // Display the User-Agent sent by the browser or HTTP client.
                <p>
                    "user agent: "
                    (user_agent(cx))
                </p>
            </body>
        </html>
    }
}
