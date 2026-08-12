use topcoat::{
    Result,
    context::Cx,
    router::{CatchAllSegments, Router, error::RouterErrorExt, page, path_param},
    view::view,
};

mod common;
use common::send;

// A parsed (non-`&str`) path parameter: the `{post_id}` segment is parsed as a
// `u32` via `FromStr`, with the parse error handled at the call site.
path_param!(post_id: u32);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let id = path_param::<PostId>(cx).ok_or_bad_request("post_id must be a number")?;
    view! {
        "post "
        (id)
    }
}

// A borrowed (`str`) path parameter: the raw segment is exposed without parsing.
path_param!(slug);

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
path_param!(user_id: u32, error = not_found);

#[page("/users/{user_id}")]
async fn user(cx: &Cx) -> Result {
    let id = path_param::<UserId>(cx)?;
    view! {
        "user "
        (id)
    }
}

// A bare `error = bad_request` prefills the description with the parameter name.
path_param!(item_id: u32, error = bad_request);

#[page("/items/{item_id}")]
async fn item(cx: &Cx) -> Result {
    let id = path_param::<ItemId>(cx)?;
    view! {
        "item "
        (id)
    }
}

// `error = bad_request("...")` overrides the prefilled description.
path_param!(
    order_id: u32,
    error = bad_request("order_id must be a number"),
);

#[page("/orders/{order_id}")]
async fn order(cx: &Cx) -> Result {
    let id = path_param::<OrderId>(cx)?;
    view! {
        "order "
        (id)
    }
}

// `error = redirect("...")` sends the client elsewhere on a failed parse.
path_param!(page_id: u32, error = redirect("/pages"));

#[page("/pages/{page_id}")]
async fn page_detail(cx: &Cx) -> Result {
    let id = path_param::<PageId>(cx)?;
    view! {
        "page "
        (id)
    }
}

path_param!(*doc_path);

#[page("/docs/{*doc_path}")]
async fn document(cx: &Cx) -> Result {
    let path: CatchAllSegments<'_> = path_param::<DocPath>(cx);
    let path = path.collect::<Vec<_>>().join("][");
    let path = format!("[{path}]");
    view! { (&path) }
}

path_param!(*number_path: u32, error = bad_request);

#[page("/numbers/{*number_path}")]
async fn numbers(cx: &Cx) -> Result {
    let number_values = path_param::<NumberPath>(cx)?;
    view! { (format!("{number_values:?}")) }
}

path_param!(*raw_numbers: u32);

#[page("/raw-numbers/{*raw_numbers}")]
async fn raw_numbers(cx: &Cx) -> Result {
    let number_values =
        path_param::<RawNumbers>(cx).ok_or_bad_request("every segment must be a number")?;
    view! { (format!("{number_values:?}")) }
}

path_param!(missing: u32);

#[page("/missing-param")]
async fn missing_param(cx: &Cx) -> Result {
    let _ = path_param::<Missing>(cx);
    view! { "unreachable" }
}

path_param!(pub public_id: u32);
path_param!(pub public_slug);
path_param!(pub *public_parts);

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

#[tokio::test]
async fn reads_decoded_catch_all_segments() {
    let router = Router::builder().page(document).build();
    let (status, body) = send(&router, "/docs/guides%2Fintroduction/getting%20started").await;
    assert_eq!(status, 200);
    assert_eq!(body, "[guides/introduction][getting started]");
}

#[tokio::test]
async fn requires_a_segment_for_a_catch_all() {
    let router = Router::builder().page(document).build();
    let (status, _) = send(&router, "/docs").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn parses_catch_all_segments() {
    let router = Router::builder().page(numbers).build();
    let (status, body) = send(&router, "/numbers/1/2/3").await;
    assert_eq!(status, 200);
    assert_eq!(body, "[1, 2, 3]");
}

#[tokio::test]
async fn reports_failing_catch_all_segment_index() {
    let router = Router::builder().page(numbers).build();
    let (status, body) = send(&router, "/numbers/1/nope/3").await;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        "bad request: invalid value for path parameter \"number_path\" at segment 1"
    );
}

#[tokio::test]
async fn returns_catch_all_parse_error_to_call_site() {
    let router = Router::builder().page(raw_numbers).build();
    let (status, body) = send(&router, "/raw-numbers/1/nope/3").await;
    assert_eq!(status, 400);
    assert_eq!(body, "bad request: every segment must be a number");
}

#[tokio::test]
async fn missing_parameter_panics() {
    let router = Router::builder().page(missing_param).build();
    // The router isolates handler panics, so the panic surfaces as a 500.
    let (status, body) = send(&router, "/missing-param").await;
    assert_eq!(status, 500);
    assert_eq!(body, "internal server error");
}

#[test]
fn constructs_public_parameter_values() {
    let id = PublicId(42);
    assert_eq!(id.0, 42);

    let borrowed = PublicSlug("rust");
    assert_eq!(borrowed.0, "rust");
    let owned: PublicSlug = PublicSlug("rust".to_owned());
    assert_eq!(owned.0, "rust");

    let borrowed = PublicParts(["guides", "start"]);
    assert_eq!(borrowed.0, ["guides", "start"]);
    let owned: PublicParts = PublicParts(vec!["guides".to_owned(), "start".to_owned()]);
    assert_eq!(owned.0, ["guides", "start"]);
}
