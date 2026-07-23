use topcoat::{
    Result,
    router::{Body, Router, route, to_bytes},
};

/// Dispatches a request with the given method through the router and returns
/// the response's status code and body as a `String`.
async fn send(router: &Router, method: &str, path: &str) -> (u16, String) {
    let request = http::Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = router.handle(request).await;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[route(GET "/one")]
async fn one() -> Result<&'static str> {
    Ok("one")
}

#[route([GET, POST] "/multi")]
async fn multi() -> Result<&'static str> {
    Ok("multi")
}

#[route(* "/any")]
async fn any() -> Result<&'static str> {
    Ok("any")
}

// A specific method and `*` at the same path: the specific method wins.
#[route(GET "/mixed")]
async fn mixed_get() -> Result<&'static str> {
    Ok("get")
}

#[route(* "/mixed")]
async fn mixed_rest() -> Result<&'static str> {
    Ok("rest")
}

#[tokio::test]
async fn a_single_method_serves_only_that_method() {
    let router = Router::builder().route(one).build();
    assert_eq!(send(&router, "GET", "/one").await.0, 200);
    assert_eq!(send(&router, "POST", "/one").await.0, 405);
}

#[tokio::test]
async fn a_method_list_serves_each_listed_method() {
    let router = Router::builder().route(multi).build();
    assert_eq!(send(&router, "GET", "/multi").await, (200, "multi".into()));
    assert_eq!(send(&router, "POST", "/multi").await, (200, "multi".into()));
    assert_eq!(send(&router, "DELETE", "/multi").await.0, 405);
}

#[tokio::test]
async fn a_star_serves_every_method() {
    let router = Router::builder().route(any).build();
    for method in ["GET", "POST", "PUT", "DELETE", "PATCH", "PURGE"] {
        assert_eq!(send(&router, method, "/any").await, (200, "any".into()));
    }
}

#[tokio::test]
async fn a_specific_method_wins_over_a_star_route() {
    let router = Router::builder().route(mixed_get).route(mixed_rest).build();
    assert_eq!(send(&router, "GET", "/mixed").await, (200, "get".into()));
    assert_eq!(send(&router, "POST", "/mixed").await, (200, "rest".into()));
}
