use topcoat::{
    Result,
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, page,
        request::{headers, uri},
    },
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

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
    // `Cx` carries the current request and can be passed to plain functions.
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Cx"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Cx functions"</h1>

                <p>
                    "path: "
                    (current_path(cx))
                </p>

                <p>
                    "user agent: "
                    (user_agent(cx))
                </p>
            </body>
        </html>
    }
}
