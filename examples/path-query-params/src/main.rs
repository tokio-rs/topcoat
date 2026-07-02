use topcoat::{
    Result,
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, RouterErrorExt, Slot, layout, page, path_param,
        query_params,
    },
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

// --- Layout -----------------------------------------------------------------

// The root layout wraps every page because every path starts with "/".
#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body>(slot.await?)</body>
        </html>
    }
}

// --- Home -------------------------------------------------------------------

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"Path and query params"</h1>
        <ul>
            <li><a href="/posts?page=2&q=rust">"query params: /posts?page=2&q=rust"</a>
            </li>
            <li><a href="/posts/42">"path param: /posts/42"</a></li>
        </ul>
    }
}

// --- Query params -----------------------------------------------------------

// #[query_params] parses URL query strings into a typed struct.
#[query_params]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
    let query = query_params::<PostsQuery>(cx).ok_or_bad_request("invalid query string")?;

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

// #[path_param] reads a matching {post_id} URL segment and parses it as u32.
#[path_param]
struct PostId(u32);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx).ok_or_bad_request("post_id must be a number")?;

    view! {
        <h1>
            "Post "
            (post_id.to_string())
        </h1>
        <p>"parsed from the {post_id} path segment"</p>
        <p><a href="/posts?page=1">"all posts"</a></p>
    }
}
