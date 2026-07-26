use topcoat::{
    Result,
    context::Cx,
    router::{Router, error::RouterErrorExt, page, path_param},
    view::view,
};

mod common;
use common::send;

// A parsed (non-`&str`) path parameter: the `{post_id}` segment is parsed as a
// `u32` via `FromStr`, with the parse error handled at the call site.
#[path_param]
struct PostId(u32);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let id = path_param::<PostId>(cx).ok_or_bad_request("post_id must be a number")?;
    view! {
        "post "
        (id)
    }
}

// A borrowed (`str`) path parameter: the raw segment is exposed without parsing.
#[path_param]
struct Slug(str);

#[page("/tags/{slug}")]
async fn tag(cx: &Cx) -> Result {
    let slug = path_param::<Slug>(cx);
    view! {
        "tag "
        (slug)
    }
}

// `error = ...` declares the response for a failed parse on the parameter
// itself, so handlers propagate it with `?`.
#[path_param(error = not_found)]
struct UserId(u32);

#[page("/users/{user_id}")]
async fn user(cx: &Cx) -> Result {
    let id = path_param::<UserId>(cx)?;
    view! {
        "user "
        (id)
    }
}

// A bare `error = bad_request` prefills the description with the parameter name.
#[path_param(error = bad_request)]
struct ItemId(u32);

#[page("/items/{item_id}")]
async fn item(cx: &Cx) -> Result {
    let id = path_param::<ItemId>(cx)?;
    view! {
        "item "
        (id)
    }
}

// `error = bad_request("...")` overrides the prefilled description.
#[path_param(error = bad_request("order_id must be a number"))]
struct OrderId(u32);

#[page("/orders/{order_id}")]
async fn order(cx: &Cx) -> Result {
    let id = path_param::<OrderId>(cx)?;
    view! {
        "order "
        (id)
    }
}

// `error = redirect("...")` sends the client elsewhere on a failed parse.
#[path_param(error = redirect("/pages"))]
struct PageId(u32);

#[page("/pages/{page_id}")]
async fn page_detail(cx: &Cx) -> Result {
    let id = path_param::<PageId>(cx)?;
    view! {
        "page "
        (id)
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

#[tokio::test]
async fn parses_param_with_declared_error() {
    let router = Router::builder().page(user).build();
    let (status, body) = send(&router, "/users/7").await;
    assert_eq!(status, 200);
    assert_eq!(body, "user 7");
}

#[tokio::test]
async fn responds_with_declared_not_found() {
    let router = Router::builder().page(user).build();
    let (status, _) = send(&router, "/users/not-a-number").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn prefills_bad_request_description() {
    let router = Router::builder().page(item).build();
    let (status, body) = send(&router, "/items/not-a-number").await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        "bad request: invalid value for path parameter \"item_id\""
    );
}

#[tokio::test]
async fn overrides_bad_request_description() {
    let router = Router::builder().page(order).build();
    let (status, body) = send(&router, "/orders/not-a-number").await;
    assert_eq!(status, 400);
    assert_eq!(body, "bad request: order_id must be a number");
}

#[tokio::test]
async fn redirects_on_failed_parse() {
    let router = Router::builder().page(page_detail).build();
    let (status, _) = send(&router, "/pages/not-a-number").await;
    assert_eq!(status, 307);
}
