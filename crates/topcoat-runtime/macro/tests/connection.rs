//! Page renders over a runtime connection.
//!
//! A WebSocket handshake at a page's URL requesting the `topcoat-runtime`
//! subprotocol opens a connection. Each render request over it runs the
//! page as a connected `GET` and streams the page's frames back.

use std::{io, net::SocketAddr, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Router,
        error::{bad_request, redirect},
        page, to_bytes,
    },
    runtime::{RUNTIME_PROTOCOL, RouterBuilderRuntimeExt, connected, signal},
    view::{View, emit, live, view},
};

/// The connection's page: shows whether the render is connected and the
/// current value of its signal, and emits twice into a live region.
#[page("/room")]
async fn room(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("initial"));
    let current = query.get();
    let connected = connected(cx);
    Ok(view! {
        <p>"connected: " (connected)</p>
        <p>"query: " (current)</p>
        <main>
            (live! {
                emit! { <p>"one"</p> }?;
                emit! { <p>"two"</p> }
            })
        </main>
    })
}

/// A page whose live region never finishes once connected.
#[page("/slow")]
async fn slow(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <main>
            (live! {
                let token = emit! { <p>"one"</p> }?;
                if connected(cx) {
                    std::future::pending::<()>().await;
                }
                Ok(token)
            })
        </main>
    })
}

/// A page that redirects before producing content.
#[page("/away")]
async fn away() -> Result<impl View> {
    let redirected: Result<()> = Err(redirect("/target").into());
    redirected?;
    Ok(view! { <p>"never"</p> })
}

/// A page that fails before producing content.
#[page("/broken")]
async fn broken() -> Result<impl View> {
    let failed: Result<()> = Err(bad_request("broken").into());
    failed?;
    Ok(view! { <p>"never"</p> })
}

fn router() -> Router {
    Router::builder()
        .page(room)
        .page(slow)
        .page(away)
        .page(broken)
        .runtime()
        .build()
}

/// Serves the router on an ephemeral port, shutting down when the returned
/// sender fires.
async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(topcoat::serve_until(listener, router(), async {
        let _ = shutdown_rx.await;
    }));
    (addr, shutdown_tx, server)
}

/// Waits for the server to return, bounded so a stuck shutdown fails the
/// test instead of hanging it.
async fn shut_down(shutdown_tx: oneshot::Sender<()>, server: JoinHandle<io::Result<()>>) {
    shutdown_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server did not shut down within the grace period")
        .unwrap()
        .unwrap();
}

type Client = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Opens a runtime connection at `path`.
async fn connect(addr: SocketAddr, path: &str) -> Client {
    let mut request = format!("ws://{addr}{path}").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("sec-websocket-protocol", RUNTIME_PROTOCOL.parse().unwrap());
    let (client, response) = tokio_tungstenite::connect_async(request)
        .await
        .expect("the handshake succeeds");
    assert_eq!(response.status(), 101);
    assert_eq!(
        response
            .headers()
            .get("sec-websocket-protocol")
            .map(|value| value.to_str().unwrap()),
        Some(RUNTIME_PROTOCOL)
    );
    client
}

/// Sends a render request for run `run` with the given signals JSON.
async fn request_run(client: &mut Client, run: u64, signals: &str) {
    client
        .send(Message::text(format!(
            "{{\"run\":{run},\"signals\":{signals}}}"
        )))
        .await
        .unwrap();
}

/// Reads the next text message as JSON, bounded so a missing message fails
/// the test instead of hanging it.
async fn next_json(client: &mut Client) -> serde_json::Value {
    let message = tokio::time::timeout(Duration::from_secs(5), client.next())
        .await
        .expect("a message arrives")
        .expect("the connection is open")
        .unwrap();
    let Message::Text(text) = message else {
        panic!("expected a text message, got {message:?}");
    };
    serde_json::from_str(text.as_str()).unwrap()
}

/// The id of the last signal declared in `html`.
fn last_signal_id(html: &str) -> &str {
    let declaration = html.rfind("::topcoat::signal(").expect(html);
    let key = "&quot;id&quot;:&quot;";
    let start = html[declaration..].find(key).expect(html) + declaration + key.len();
    let end = html[start..].find("&quot;").expect(html) + start;
    &html[start..end]
}

#[tokio::test]
async fn a_run_renders_the_page_connected_and_streams_its_frames() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_run(&mut client, 1, "{}").await;

    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "run", "id": 1 })
    );
    let snapshot = next_json(&mut client).await;
    assert_eq!(snapshot["t"], "snapshot");
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("connected: true"), "{html}");
    assert!(html.contains("query: initial"), "{html}");
    assert!(html.contains("<!--::topcoat::connect-->"), "{html}");
    assert!(html.contains("<p>one</p>"), "{html}");
    // The live region's second emission follows as a swap for its region.
    let swap = next_json(&mut client).await;
    assert_eq!(swap["t"], "swap");
    assert_eq!(swap["html"], "<p>two</p>");
    let region = swap["region"].as_str().unwrap();
    assert!(
        html.contains(&format!("::topcoat::region::start({region})")),
        "{html}"
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_run_restores_the_signal_values_it_is_sent() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_run(&mut client, 1, "{}").await;
    let _run = next_json(&mut client).await;
    let snapshot = next_json(&mut client).await;
    let id = last_signal_id(snapshot["html"].as_str().unwrap()).to_owned();
    let _swap = next_json(&mut client).await;

    request_run(&mut client, 2, &format!("{{\"{id}\":\"changed\"}}")).await;
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "run", "id": 2 })
    );
    let snapshot = next_json(&mut client).await;
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("query: changed"), "{html}");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_new_run_supersedes_a_run_that_never_finishes() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/slow").await;

    request_run(&mut client, 1, "{}").await;
    assert_eq!(next_json(&mut client).await["t"], "run");
    assert_eq!(next_json(&mut client).await["t"], "snapshot");

    // The first run is still pending in its live body when the second
    // arrives; the second's output follows without anything in between.
    request_run(&mut client, 2, "{}").await;
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "run", "id": 2 })
    );
    assert_eq!(next_json(&mut client).await["t"], "snapshot");

    // Closing while a run is pending still tears the connection down.
    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_redirecting_page_sends_a_redirect_message() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/away").await;

    request_run(&mut client, 1, "{}").await;
    assert_eq!(next_json(&mut client).await["t"], "run");
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "redirect", "location": "/target" })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_failing_page_sends_an_error_message_with_its_status() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/broken").await;

    request_run(&mut client, 1, "{}").await;
    assert_eq!(next_json(&mut client).await["t"], "run");
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "error", "status": 400 })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_malformed_render_request_is_answered_and_the_connection_stays_open() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    client.send(Message::text("not json")).await.unwrap();
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "t": "error", "status": 400 })
    );

    request_run(&mut client, 1, "{}").await;
    assert_eq!(next_json(&mut client).await["t"], "run");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

/// Builds a handshake for `/room` with the given extra headers.
fn handshake(extra: &[(&str, &str)]) -> http::Request<Body> {
    let mut request = http::Request::builder()
        .method("GET")
        .uri("/room")
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==");
    for (name, value) in extra {
        request = request.header(*name, *value);
    }
    request.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn a_handshake_without_the_runtime_protocol_reaches_the_page() {
    let response = router().handle(handshake(&[])).await;
    assert_eq!(response.status(), 200);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("connected: false"), "{html}");
}

#[tokio::test]
async fn a_runtime_handshake_on_a_connection_that_cannot_upgrade_is_a_bad_request() {
    // Dispatched directly, the request carries no upgrade handle.
    let response = router()
        .handle(handshake(&[("sec-websocket-protocol", RUNTIME_PROTOCOL)]))
        .await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn a_cross_site_runtime_handshake_is_refused_before_the_layer_runs() {
    let response = router()
        .handle(handshake(&[
            ("sec-websocket-protocol", RUNTIME_PROTOCOL),
            ("host", "app.example"),
            ("origin", "https://evil.example"),
        ]))
        .await;
    assert!(response.status().is_client_error());
    assert_ne!(response.status(), 400);
}
