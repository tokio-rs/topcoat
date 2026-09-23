use topcoat::{
    Result,
    router::{Body, Route, Router, response::Response, to_bytes},
    runtime::{Surrogated, procedure},
};

#[procedure]
async fn without_arguments() -> Result<bool> {
    Ok(true)
}

#[procedure("/api/double")]
async fn at_path(value: f64) -> Result<f64> {
    Ok(value * 2.0)
}

#[procedure("/(api)/grouped")]
async fn grouped(value: f64) -> Result<f64> {
    Ok(value)
}

#[procedure]
async fn with_unit(_value: ()) -> Result<bool> {
    Ok(true)
}

#[procedure]
async fn with_arguments(enabled: bool, label: String) -> Result<String> {
    Ok(if enabled { label } else { String::new() })
}

/// Calls a procedure through the router at its served URL.
async fn call(procedure: &'static dyn Route, body: &'static str) -> Response {
    let request = http::Request::builder()
        .method("POST")
        .uri(procedure.path().to_matchit_path().as_ref())
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Router::builder()
        .route(procedure)
        .build()
        .handle(request)
        .await
}

#[tokio::test]
async fn a_procedure_without_a_path_is_served_below_the_runtime_prefix() {
    let path = without_arguments.path().as_str();
    let tail = path
        .strip_prefix("/_topcoat/runtime/procedures/")
        .expect(path);
    assert_eq!(tail.len(), 32, "{path}");
    assert!(tail.bytes().all(|b| b.is_ascii_hexdigit()), "{path}");
    assert_ne!(without_arguments.path(), with_unit.path());
}

#[tokio::test]
async fn a_procedure_with_a_path_is_served_there_and_its_surrogate_names_it() {
    assert_eq!(at_path.path().as_str(), "/api/double");

    let response = call(&at_path, "[2.5]").await;
    assert_eq!(response.status(), http::StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], b"5.0");

    let surrogate = serde_json::to_value(Surrogated::into_surrogate(at_path)).unwrap();
    assert_eq!(surrogate["t"], "Procedure");
    assert_eq!(surrogate["path"], "/api/double");
}

#[tokio::test]
async fn a_grouped_path_serializes_as_the_served_url() {
    // The router strips the group from the URL it serves, so the browser
    // must post to the stripped form.
    let surrogate = serde_json::to_value(Surrogated::into_surrogate(grouped)).unwrap();
    assert_eq!(surrogate["path"], "/grouped");

    let response = call(&grouped, "[1.5]").await;
    assert_eq!(response.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn empty_arguments_and_one_unit_argument_are_distinct() {
    for (procedure, body) in [
        (&without_arguments as &'static dyn Route, "[]"),
        (&with_unit as &'static dyn Route, "[null]"),
    ] {
        let response = call(procedure, body).await;
        assert_eq!(response.status(), http::StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&bytes[..], b"true");
    }

    for body in ["null", "[null]"] {
        assert_eq!(
            call(&without_arguments, body).await.status(),
            http::StatusCode::BAD_REQUEST,
        );
    }
    assert_eq!(
        call(&with_unit, "[]").await.status(),
        http::StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn multiple_arguments_keep_their_order_and_types() {
    let response = call(&with_arguments, r#"[true,"hello"]"#).await;
    assert_eq!(response.status(), http::StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], br#""hello""#);

    for body in [r#"["hello",true]"#, "[true]", r#"[true,"hello",null]"#] {
        assert_eq!(
            call(&with_arguments, body).await.status(),
            http::StatusCode::BAD_REQUEST
        );
    }
}
