use std::net::SocketAddr;

use tokio::net::TcpListener;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Json, Router, Slot, layout, page, route},
    view::view,
};

#[layout("/")]
async fn root(slot: Slot<'_>) -> Result {
    view! {
        <html><body>(slot.await?)</body></html>
    }
}

#[page("/")]
async fn home() -> Result {
    view! { <h1>"home"</h1> }
}

#[page("/about")]
async fn about() -> Result {
    view! { <h1>"about"</h1> }
}

#[page("/users/{id}")]
async fn user_profile() -> Result {
    view! { <h1>"profile"</h1> }
}

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Greeting {
    name: String,
}

#[route(POST "/api/echo")]
async fn echo(Json(input): Json<Greeting>) -> Result<Json<Greeting>> {
    Ok(Json(input))
}

struct Marker(&'static str);

#[page("/state")]
async fn state_page(cx: &Cx) -> Result {
    let marker = app_context::<Marker>(cx);
    view! { <p>(marker.0)</p> }
}

fn test_router() -> Router {
    Router::new()
        .layout(root)
        .page(home)
        .page(about)
        .page(user_profile)
        .page(state_page)
        .route(health)
        .route(echo)
        .app_context(Marker("hello-from-state"))
}

async fn spawn_server(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        topcoat::serve(listener, router).await.unwrap();
    });
    addr
}

fn url(addr: SocketAddr, path: &str) -> String {
    format!("http://{addr}{path}")
}

#[tokio::test]
async fn server_serves_home_page_wrapped_in_root_layout() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/")).await.unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("<html>"));
    assert!(body.contains("<h1>home</h1>"));
}

#[tokio::test]
async fn server_serves_second_page_with_distinct_body() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/about")).await.unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("<h1>about</h1>"));
}

#[tokio::test]
async fn server_matches_dynamic_path_segments() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/users/42")).await.unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.text().await.unwrap().contains("<h1>profile</h1>"));
}

#[tokio::test]
async fn server_returns_404_for_unknown_path() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/does-not-exist")).await.unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn api_route_returns_plain_string_body() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/api/health")).await.unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn api_route_round_trips_json_body() {
    let addr = spawn_server(test_router()).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(url(addr, "/api/echo"))
        .json(&Greeting {
            name: "Ada".to_owned(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let echoed: Greeting = resp.json().await.unwrap();
    assert_eq!(echoed.name, "Ada");
}

#[tokio::test]
async fn page_can_read_registered_app_context() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::get(url(addr, "/state")).await.unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.text().await.unwrap().contains("hello-from-state"));
}

#[tokio::test]
async fn wrong_method_on_route_returns_method_not_allowed() {
    let addr = spawn_server(test_router()).await;
    let resp = reqwest::Client::new()
        .post(url(addr, "/api/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
}
