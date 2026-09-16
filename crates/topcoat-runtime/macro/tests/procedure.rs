use topcoat::{
    Result,
    router::{Body, Route, Router, response::Response, to_bytes},
    runtime::{Procedure, ProcedureRoute, procedure},
};

#[procedure]
async fn without_arguments() -> Result<bool> {
    Ok(true)
}

#[procedure]
async fn with_unit(_value: ()) -> Result<bool> {
    Ok(true)
}

#[procedure]
async fn with_arguments(enabled: bool, label: String) -> Result<String> {
    Ok(if enabled { label } else { String::new() })
}

async fn call(procedure: &'static dyn Procedure, body: &'static str) -> Response {
    let route = ProcedureRoute::new(procedure);
    let request = http::Request::builder()
        .method("POST")
        .uri(route.path().as_str())
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Router::builder().route(route).build().handle(request).await
}

#[tokio::test]
async fn empty_arguments_and_one_unit_argument_are_distinct() {
    for (procedure, body) in [
        (&without_arguments as &'static dyn Procedure, "[]"),
        (&with_unit as &'static dyn Procedure, "[null]"),
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
    assert_eq!(call(&with_unit, "[]").await.status(), http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn multiple_arguments_keep_their_order_and_types() {
    let response = call(&with_arguments, r#"[true,"hello"]"#).await;
    assert_eq!(response.status(), http::StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], br#""hello""#);

    for body in [r#"["hello",true]"#, "[true]", r#"[true,"hello",null]"#] {
        assert_eq!(call(&with_arguments, body).await.status(), http::StatusCode::BAD_REQUEST);
    }
}
