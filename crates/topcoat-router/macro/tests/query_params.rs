use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterErrorExt, page, query_params},
    view::view,
};

mod common;
use common::{send, send_full};

// A plain query struct: the parse error is handled at the call site.
#[query_params]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
    let query = query_params::<PostsQuery>(cx).ok_or_bad_request("invalid query string")?;
    view! {
        "page="
        (query.page.unwrap_or(1).to_string())
        " q="
        (query.q.as_deref().unwrap_or("all"))
    }
}

// `error = ...` declares the response for a failed parse on the struct
// itself, so handlers propagate it with `?`.
#[query_params(error = bad_request)]
struct SearchQuery {
    limit: u32,
}

#[page("/search")]
async fn search(cx: &Cx) -> Result {
    let query = query_params::<SearchQuery>(cx)?;
    view! {
        "limit="
        (query.limit)
    }
}

// `error = redirect("?")` reloads the page with the query string cleared.
#[query_params(error = redirect("?"))]
struct FilterQuery {
    min: Option<u32>,
}

#[page("/filter")]
async fn filter(cx: &Cx) -> Result {
    let query = query_params::<FilterQuery>(cx)?;
    view! {
        "min="
        (query.min.unwrap_or(0))
    }
}

#[tokio::test]
async fn parses_query_string() {
    let router = Router::builder().page(posts).build();
    let (status, body) = send(&router, "/posts?page=2&q=rust").await;
    assert_eq!(status, 200);
    assert_eq!(body, "page=2 q=rust");
}

#[tokio::test]
async fn absent_optional_fields_default_to_none() {
    let router = Router::builder().page(posts).build();
    let (status, body) = send(&router, "/posts").await;
    assert_eq!(status, 200);
    assert_eq!(body, "page=1 q=all");
}

#[tokio::test]
async fn reports_invalid_query_string() {
    let router = Router::builder().page(posts).build();
    // `page` expects a `u32`, so a non-numeric value fails to deserialize.
    let (status, _) = send(&router, "/posts?page=abc").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn parses_query_with_declared_error() {
    let router = Router::builder().page(search).build();
    let (status, body) = send(&router, "/search?limit=5").await;
    assert_eq!(status, 200);
    assert_eq!(body, "limit=5");
}

#[tokio::test]
async fn reports_failing_query_param() {
    let router = Router::builder().page(search).build();
    let (status, body) = send(&router, "/search?limit=abc").await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        "bad request: invalid query value: invalid digit found in string (at `limit`)"
    );
}

#[tokio::test]
async fn redirects_with_query_cleared() {
    let router = Router::builder().page(filter).build();
    let (status, headers, _) = send_full(&router, "/filter?min=abc").await;
    assert_eq!(status, 307);
    assert_eq!(headers[http::header::LOCATION], "?");
}
