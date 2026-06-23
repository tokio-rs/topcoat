use topcoat::router::{Body, Router, to_bytes};

/// Dispatches a `GET` request through the router and returns the response's
/// status code and body as a `String`.
pub async fn send(router: &Router, path: &str) -> (u16, String) {
    let request = http::Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = router.handle(request).await;
    let status = response.status().as_u16();
    let (_, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}
