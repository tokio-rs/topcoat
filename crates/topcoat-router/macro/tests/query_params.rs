use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterErrorExt, page, query_params},
    view::view,
};

mod common;
use common::send;

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
