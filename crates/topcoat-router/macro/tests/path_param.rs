use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterErrorExt, page, path_param},
    view::view,
};

mod common;
use common::send;

// A parsed (non-`&str`) path parameter: the `{post_id}` segment is parsed as a
// `u32` via `FromStr`.
#[path_param]
struct PostId(u32);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let id = path_param::<PostId>(cx).ok_or_bad_request("post_id must be a number")?;
    view! {
        "post "
        (id.to_string())
    }
}

// A borrowed (`&str`) path parameter: the raw segment is exposed without parsing.
#[path_param]
struct Slug<'a>(&'a str);

#[page("/tags/{slug}")]
async fn tag(cx: &Cx) -> Result {
    let slug = path_param::<Slug>(cx);
    view! {
        "tag "
        (&**slug)
    }
}

#[tokio::test]
async fn parses_typed_path_param() {
    let router = Router::builder().page(post).build();
    let (status, body) = send(&router, "/posts/42").await;
    assert_eq!(status, 200);
    assert_eq!(body, "post 42");
}

#[tokio::test]
async fn reports_unparsable_path_param() {
    let router = Router::builder().page(post).build();
    let (status, _) = send(&router, "/posts/not-a-number").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn borrows_str_path_param() {
    let router = Router::builder().page(tag).build();
    let (status, body) = send(&router, "/tags/rust").await;
    assert_eq!(status, 200);
    assert_eq!(body, "tag rust");
}

#[tokio::test]
async fn decodes_percent_encoded_str_param() {
    let router = Router::builder().page(tag).build();
    let (status, body) = send(&router, "/tags/hello%20world").await;
    assert_eq!(status, 200);
    assert_eq!(body, "tag hello world");
}
