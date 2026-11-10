//! Documents routes declared with the handler macros and serves the result.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use topcoat::{
    Result,
    context::Cx,
    openapi::{Document, OpenApi, Operation, Parameter},
    router::{
        Body, Method, Next, Router, StatusCode, content::Json, layer, path_param, query_params,
        request::Request, response::Response, route, router, to_bytes,
    },
};

#[derive(Deserialize, JsonSchema)]
struct CreateUser {
    name: String,
}

#[derive(Serialize, JsonSchema)]
struct User {
    id: u64,
    name: String,
}

#[query_params]
#[derive(JsonSchema)]
struct ListQuery {
    page: Option<u32>,
}

path_param!(user_id: u64);

#[layer("/api", with(Operation::new().tag("api")))]
async fn api(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}

#[route(GET "/api/users", with(Operation::new().query::<ListQuery>()))]
async fn list_users(cx: &Cx) -> Result<Json<Vec<User>>> {
    let page = query_params::<ListQuery>(cx)
        .ok()
        .and_then(|query| query.page);
    let users = match page {
        None | Some(1) => vec![User {
            id: 1,
            name: "Ada".to_owned(),
        }],
        Some(_) => Vec::new(),
    };
    Ok(Json(users))
}

#[route(
    POST "/api/users",
    with(Operation::new()
        .request::<Json<CreateUser>>()
        .response_with::<Json<User>>(StatusCode::CREATED, "The new user."))
)]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<(StatusCode, Json<User>)> {
    let user = User {
        id: 1,
        name: input.name,
    };
    Ok((StatusCode::CREATED, Json(user)))
}

#[route(
    GET "/api/users/{user_id}",
    with(Operation::new().parameter(Parameter::path("user_id").schema::<u64>()))
)]
async fn show_user(cx: &Cx) -> Result<&'static str> {
    Ok("user")
}

#[route(GET "/openapi.json")]
async fn spec(cx: &Cx) -> Result<Document> {
    Ok(OpenApi::new("Users API", "1.0.0").generate(&router(cx)))
}

fn app() -> Router {
    Router::builder()
        .layer(api)
        .route(list_users)
        .route(create_user)
        .route(show_user)
        .route(spec)
        .build()
}

/// Requests the document from the route that serves it.
async fn served_document(router: &Router) -> Value {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/openapi.json")
        .body(Body::empty())
        .unwrap();
    let response = router.handle(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn a_route_serves_the_document_of_its_router() {
    let router = app();
    let document = served_document(&router).await;

    // The same document can be created without a request.
    let generated = OpenApi::new("Users API", "1.0.0").generate(&router);
    assert_eq!(document, serde_json::to_value(&generated).unwrap());
}

#[tokio::test]
async fn operations_attached_through_the_handler_macros_are_documented() {
    let document = served_document(&app()).await;
    let paths = document["paths"].as_object().unwrap();

    // The route serving the document has no operation and is outside the
    // documented layer.
    let mut documented: Vec<&str> = paths.keys().map(String::as_str).collect();
    documented.sort_unstable();
    assert_eq!(documented, ["/api/users", "/api/users/{user_id}"]);

    let users = &paths["/api/users"];
    assert_eq!(users["get"]["tags"][0], "api");
    assert_eq!(users["get"]["parameters"][0]["name"], "page");
    assert_eq!(users["get"]["parameters"][0]["in"], "query");
    assert!(users["post"]["requestBody"]["content"]["application/json"].is_object());
    assert!(users["post"]["responses"]["201"]["content"]["application/json"].is_object());

    let user_id = &paths["/api/users/{user_id}"]["get"]["parameters"][0];
    assert_eq!(user_id["in"], "path");
    assert_eq!(user_id["schema"]["type"], "integer");
}
