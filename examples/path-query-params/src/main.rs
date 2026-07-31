use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, layout, page, path_param, query_params},
    view::view,
};

#[tokio::main]
async fn main() {
    // Discover the routes declared in this example and start the server.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

// --- Layout -----------------------------------------------------------------

// The root layout wraps every page because every path starts with "/".
#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body>(slot?)</body>
        </html>
    }
}

// --- Home -------------------------------------------------------------------

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"Path and query params"</h1>
        <ul>
            <li>
                <a href="/posts?page=2&q=rust">"query params: /posts?page=2&q=rust"</a>
            </li>
            <li><a href="/posts/42">"path param: /posts/42"</a></li>
            <li>
                <a href="/docs/guides/getting-started">
                    "catch-all param: /docs/guides/getting-started"
                </a>
            </li>
        </ul>
    }
}

// --- Query params -----------------------------------------------------------

// Parse the URL query string into this typed structure.
// Invalid values redirect to the same page with the query string cleared.
#[query_params(error = redirect("?"))]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
    // Read and validate the query parameters from the current request.
    let query = query_params::<PostsQuery>(cx)?;

    view! {
        <h1>"Posts"</h1>
        <p>
            "page: "
            (query.page.unwrap_or(1))
        </p>
        <p>
            "search: "
            (query.q.as_deref().unwrap_or("all"))
        </p>
        <p><a href="/">"back home"</a></p>
    }
}

// --- Path params ------------------------------------------------------------

// Declare the {post_id} URL segment and parse it as a u32.
// Return a bad request response if the value is not a number.
path_param!(
    post_id: u32,
    error = bad_request("Post ID must be a number!"),
);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    // Extract the typed post ID from the current request path.
    let post_id = path_param::<PostId>(cx)?;

    view! {
        <h1>
            "Post "
            (post_id)
        </h1>
        <p>"parsed from the {post_id} path segment"</p>
        <p><a href="/posts?page=1">"all posts"</a></p>
    }
}

// --- Catch-all params -------------------------------------------------------

// A leading `*` captures every remaining segment. Without a type the parameter
// reads back as decoded segments.
path_param!(*doc_path);

#[page("/docs/{*doc_path}")]
async fn document(cx: &Cx) -> Result {
    view! {
        <h1>"Documentation path"</h1>
        <ul>
            for segment in path_param::<DocPath>(cx) {
                <li>(segment)</li>
            }
        </ul>
        <p><a href="/">"back home"</a></p>
    }
}
