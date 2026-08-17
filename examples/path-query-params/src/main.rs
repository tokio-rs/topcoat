use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, href, layout, page, path_param, query_params},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

// --- Layout -----------------------------------------------------------------

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
            // `href` builds the URL from the page it points at: `query` adds
            // query items, and the tuple fills the path's parameters.
            <li>
                <a href=(href!(posts).query([("page", "2"), ("q", "rust")]))>
                    "query params: /posts?page=2&q=rust"
                </a>
            </li>
            <li><a href=(href!(post, PostId(42)))>"path param: /posts/42"</a></li>
            <li>
                <a href=(href!(document, DocPath(["guides", "getting-started"])))>
                    "catch-all param: /docs/guides/getting-started"
                </a>
            </li>
        </ul>
    }
}

// --- Query params -----------------------------------------------------------

// A value that does not parse redirects to the page with the query cleared.
#[query_params(error = redirect("?"))]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
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
        <p><a href=(href!(home))>"back home"</a></p>
    }
}

// --- Path params ------------------------------------------------------------

// Declares the `{post_id}` segment and the error for a value that is no u32.
path_param!(
    post_id: u32,
    error = bad_request("Post ID must be a number!"),
);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx)?;

    view! {
        <h1>
            "Post "
            (post_id)
        </h1>
        <p>"parsed from the {post_id} path segment"</p>
        <p><a href=(href!(posts).query([("page", 1)]))>"all posts"</a></p>
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
        <p><a href=(href!(home))>"back home"</a></p>
    }
}
