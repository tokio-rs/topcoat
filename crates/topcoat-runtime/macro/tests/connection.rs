//! Tests page and shard rendering over a runtime WebSocket.
//!
//! The client connects at a page's URL using the `topcoat-runtime`
//! subprotocol. Each run request describes the HTTP request that re-renders
//! a page or shard. The server dispatches it as a connected render and sends
//! the content back as frames, each tagged with its run.

use std::{io, net::SocketAddr, sync::LazyLock, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    sync::{Notify, oneshot},
    task::JoinHandle,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use topcoat::{
    Result,
    context::Cx,
    core::identity::Identity,
    router::{
        Body, Router, RouterBuilder,
        error::{bad_request, redirect},
        page, to_bytes,
    },
    runtime::{RUNTIME_PROTOCOL, RouterBuilderRuntimeExt, connected, shard, signal},
    view::{View, emit, live, view},
};

/// Displays the connection state and a signal value, then emits two
/// updates from a live region.
#[page("/room")]
async fn room(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("initial"));
    let current = query.get();
    let connected = connected(cx);
    Ok(view! {
        <p>
            "connected: "
            (connected)
        </p>
        <p>
            "query: "
            (current)
        </p>
        <main>
            (live! {
                emit! { <p>"one"</p> }?;
                emit! { <p>"two"</p> }
            })
        </main>
    })
}

/// Keeps a live region open indefinitely during a connected render.
#[page("/slow")]
async fn slow(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <main>
            (live! {
                let token = emit! { <p>"slow"</p> }?;
                if connected(cx) {
                    std::future::pending::<()>().await;
                }
                Ok(token)
            })
        </main>
    })
}

/// Notified when the render of [`endless`] is dropped.
static ENDLESS_DROPPED: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Notifies [`ENDLESS_DROPPED`] when dropped.
struct NotifyOnDrop;

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        ENDLESS_DROPPED.notify_one();
    }
}

/// Renders until its run is stopped, reporting when that happens.
#[page("/endless")]
async fn endless() -> Result<impl View> {
    Ok(view! {
        <main>
            (live! {
                let _guard = NotifyOnDrop;
                let token = emit! { <p>"endless"</p> }?;
                std::future::pending::<()>().await;
                Ok(token)
            })
        </main>
    })
}

/// Updates a live region after its first render.
#[shard]
async fn ticker() -> Result<impl View> {
    Ok(view! {
        (live! {
            emit! { <p>"tick"</p> }?;
            emit! { <p>"tock"</p> }
        })
    })
}

/// Hosts a live shard, whose updates travel with the page's run.
#[page("/shards")]
async fn shards() -> Result<impl View> {
    Ok(view! { <main>ticker()</main> })
}

/// Shows its argument, the connection state, and a signal of its own,
/// then updates a live region while connected.
#[shard("/feed")]
async fn feed(cx: &Cx, label: String) -> Result<impl View> {
    let count = signal(cx, || 0.0);
    let current = count.get();
    let connected = connected(cx);
    Ok(view! {
        <p>
            (label)
            " connected: "
            (connected)
            " count: "
            (current)
        </p>
        (live! {
            let token = emit! { <p>"first"</p> }?;
            if connected {
                emit! { <p>"pushed"</p> }
            } else {
                Ok(token)
            }
        })
    })
}

/// Redirects before rendering any content.
#[page("/away")]
async fn away() -> Result<impl View> {
    let redirected: Result<()> = Err(redirect("/target").into());
    redirected?;
    Ok(view! { <p>"never"</p> })
}

/// Returns an error before rendering any content.
#[page("/broken")]
async fn broken() -> Result<impl View> {
    let failed: Result<()> = Err(bad_request("broken").into());
    failed?;
    Ok(view! { <p>"never"</p> })
}

fn builder() -> RouterBuilder {
    Router::builder()
        .page(room)
        .page(slow)
        .page(endless)
        .page(shards)
        .page(away)
        .page(broken)
        .route(feed)
        .runtime()
}

fn router() -> Router {
    builder().build()
}

/// Starts a server on an available port. Sending on the returned channel
/// shuts it down.
async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
    serve(router()).await
}

/// Starts a server for `router` on an available port. Sending on the
/// returned channel shuts it down.
async fn serve(router: Router) -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(topcoat::serve_until(listener, router, async {
        let _ = shutdown_rx.await;
    }));
    (addr, shutdown_tx, server)
}

/// Stops the server and fails the test if shutdown takes too long.
async fn shut_down(shutdown_tx: oneshot::Sender<()>, server: JoinHandle<io::Result<()>>) {
    shutdown_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server did not shut down within the grace period")
        .unwrap()
        .unwrap();
}

type Client = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Connects to `path` using the runtime subprotocol.
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

/// Sends `message` as JSON text.
async fn send(client: &mut Client, message: serde_json::Value) {
    client
        .send(Message::text(message.to_string()))
        .await
        .unwrap();
}

/// Requests a run of the page at `path`, as an HTTP page rerun would, with
/// the supplied signal values.
async fn request_page_run(client: &mut Client, run: u64, path: &str, signals: serde_json::Value) {
    send(
        client,
        serde_json::json!({
            "run": run,
            "method": "POST",
            "path": path,
            "headers": {
                "Content-Type": "application/json",
                "X-Topcoat-Runtime": "true",
            },
            "body": serde_json::json!({ "signals": signals }).to_string(),
        }),
    )
    .await;
}

/// Requests a run of the `/feed` shard at the root identity, as an HTTP
/// shard re-render would, with the supplied arguments and signal values.
async fn request_shard_run(
    client: &mut Client,
    run: u64,
    args: serde_json::Value,
    signals: serde_json::Value,
) {
    send(
        client,
        serde_json::json!({
            "run": run,
            "method": "POST",
            "path": "/feed",
            "headers": {
                "Content-Type": "application/json",
                "X-Topcoat-Identity": Identity::ROOT.to_string(),
            },
            "body": serde_json::json!({ "args": args, "signals": signals }).to_string(),
        }),
    )
    .await;
}

/// Parses the next text message as JSON, failing the test if it does not
/// arrive before the timeout.
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

/// Returns the next frame, asserting that it belongs to `run`.
async fn next_frame(client: &mut Client, run: u64) -> serde_json::Value {
    let mut message = next_json(client).await;
    assert_eq!(message["run"], run, "{message}");
    message["frame"].take()
}

/// Finds the last signal declaration in `html` and returns its id.
fn last_signal_id(html: &str) -> &str {
    let declaration = html.rfind("::topcoat::signal(").expect(html);
    let key = "&quot;id&quot;:&quot;";
    let start = html[declaration..].find(key).expect(html) + declaration + key.len();
    let end = html[start..].find("&quot;").expect(html) + start;
    &html[start..end]
}

#[tokio::test]
async fn a_page_run_renders_the_page_connected_and_streams_its_frames() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_page_run(&mut client, 1, "/room", serde_json::json!({})).await;

    let snapshot = next_frame(&mut client, 1).await;
    assert_eq!(snapshot["t"], "snapshot");
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("connected: true"), "{html}");
    assert!(html.contains("query: initial"), "{html}");
    assert!(html.contains("<!--::topcoat::connect-->"), "{html}");
    assert!(html.contains("<p>one</p>"), "{html}");
    // The second emission must update the region from the initial HTML.
    let swap = next_frame(&mut client, 1).await;
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
async fn a_live_shard_streams_its_updates_with_the_page_run() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/shards").await;

    request_page_run(&mut client, 1, "/shards", serde_json::json!({})).await;

    let snapshot = next_frame(&mut client, 1).await;
    assert_eq!(snapshot["t"], "snapshot");
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("<p>tick</p>"), "{html}");
    assert!(!html.contains("<p>tock</p>"), "{html}");

    // The shard's region updates through the page's run, and its markers
    // lie inside the shard's, so the browser attributes it to the shard.
    let swap = next_frame(&mut client, 1).await;
    assert_eq!(swap["t"], "swap");
    assert!(swap["html"].as_str().unwrap().contains("<p>tock</p>"));
    let region = swap["region"].as_str().unwrap();
    let region_start = html
        .find(&format!("::topcoat::region::start({region})"))
        .expect(html);
    let shard_start = html.find("::topcoat::shard::start(").expect(html);
    let shard_end = html.find("::topcoat::shard::end(").expect(html);
    assert!(
        shard_start < region_start && region_start < shard_end,
        "{html}"
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_shard_run_on_a_page_connection_renders_the_shard_endpoint_connected() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_shard_run(
        &mut client,
        1,
        serde_json::json!(["news"]),
        serde_json::json!({}),
    )
    .await;

    let snapshot = next_frame(&mut client, 1).await;
    assert_eq!(snapshot["t"], "snapshot", "{snapshot}");
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("news connected: true"), "{html}");
    assert!(html.contains("<!--::topcoat::connect-->"), "{html}");
    // The endpoint renders the shard's content without its scope markers.
    assert!(!html.contains("::topcoat::shard::start("), "{html}");
    let swap = next_frame(&mut client, 1).await;
    assert_eq!(swap["t"], "swap");
    assert_eq!(swap["html"], "<p>pushed</p>");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn runs_proceed_side_by_side_on_one_connection() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/slow").await;

    // The page's run never finishes while connected.
    request_page_run(&mut client, 1, "/slow", serde_json::json!({})).await;
    let snapshot = next_frame(&mut client, 1).await;
    assert!(snapshot["html"].as_str().unwrap().contains("slow"));

    // A shard run starts without ending the page's run.
    request_shard_run(
        &mut client,
        2,
        serde_json::json!(["news"]),
        serde_json::json!({}),
    )
    .await;
    let snapshot = next_frame(&mut client, 2).await;
    assert!(snapshot["html"].as_str().unwrap().contains("news"));
    assert_eq!(next_frame(&mut client, 2).await["t"], "swap");

    // Further runs keep starting while the first page run is still going.
    request_page_run(&mut client, 3, "/slow", serde_json::json!({})).await;
    assert_eq!(next_frame(&mut client, 3).await["t"], "snapshot");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn stopping_a_run_drops_its_render() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/endless").await;

    request_page_run(&mut client, 1, "/endless", serde_json::json!({})).await;
    assert_eq!(next_frame(&mut client, 1).await["t"], "snapshot");

    send(&mut client, serde_json::json!({ "stop": 1 })).await;
    tokio::time::timeout(Duration::from_secs(5), ENDLESS_DROPPED.notified())
        .await
        .expect("the stopped render is dropped");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_connection_refuses_new_runs_beyond_its_limit_until_one_stops() {
    let (addr, shutdown_tx, server) = serve(builder().max_runs_per_connection(2).build()).await;
    let mut client = connect(addr, "/slow").await;

    // Both runs stay open while connected.
    for run in [1, 2] {
        request_page_run(&mut client, run, "/slow", serde_json::json!({})).await;
        assert_eq!(next_frame(&mut client, run).await["t"], "snapshot");
    }

    request_page_run(&mut client, 3, "/slow", serde_json::json!({})).await;
    assert_eq!(
        next_frame(&mut client, 3).await,
        serde_json::json!({ "t": "error", "status": 429 })
    );

    // Reusing an id replaces its run instead of adding one.
    request_page_run(&mut client, 2, "/slow", serde_json::json!({})).await;
    assert_eq!(next_frame(&mut client, 2).await["t"], "snapshot");

    // A stopped run frees its place.
    send(&mut client, serde_json::json!({ "stop": 1 })).await;
    request_page_run(&mut client, 4, "/slow", serde_json::json!({})).await;
    assert_eq!(next_frame(&mut client, 4).await["t"], "snapshot");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_shard_run_restores_the_signal_values_it_is_sent() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    let args = serde_json::json!(["news"]);
    request_shard_run(&mut client, 1, args.clone(), serde_json::json!({})).await;
    let snapshot = next_frame(&mut client, 1).await;
    let id = last_signal_id(snapshot["html"].as_str().unwrap()).to_owned();

    request_shard_run(&mut client, 2, args, serde_json::json!({ id: 7 })).await;
    // Frames of the first run may still arrive before the second's.
    let html = loop {
        let mut message = next_json(&mut client).await;
        if message["run"] == 2 {
            break message["frame"]["html"].take();
        }
    };
    let html = html.as_str().unwrap();
    assert!(html.contains("count: 7"), "{html}");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_shard_run_with_invalid_arguments_sends_an_error_frame() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_shard_run(
        &mut client,
        1,
        serde_json::json!([1, 2, 3]),
        serde_json::json!({}),
    )
    .await;
    let error = next_frame(&mut client, 1).await;
    assert_eq!(error["t"], "error");
    assert!(
        (400..500).contains(&error["status"].as_u64().unwrap()),
        "{error}"
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_page_run_restores_the_signal_values_it_is_sent() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    request_page_run(&mut client, 1, "/room", serde_json::json!({})).await;
    let snapshot = next_frame(&mut client, 1).await;
    let id = last_signal_id(snapshot["html"].as_str().unwrap()).to_owned();
    let _swap = next_frame(&mut client, 1).await;

    request_page_run(
        &mut client,
        2,
        "/room",
        serde_json::json!({ id: "changed" }),
    )
    .await;
    let snapshot = next_frame(&mut client, 2).await;
    let html = snapshot["html"].as_str().unwrap();
    assert!(html.contains("query: changed"), "{html}");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_run_may_only_set_runtime_headers() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    send(
        &mut client,
        serde_json::json!({
            "run": 1,
            "method": "POST",
            "path": "/room",
            "headers": { "X-Topcoat-Runtime": "true", "Cookie": "session=other" },
            "body": "{}",
        }),
    )
    .await;
    assert_eq!(
        next_frame(&mut client, 1).await,
        serde_json::json!({ "t": "error", "status": 400 })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_run_to_another_host_is_refused() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    send(
        &mut client,
        serde_json::json!({
            "run": 1,
            "method": "GET",
            "path": "https://other.example/room",
        }),
    )
    .await;
    assert_eq!(
        next_frame(&mut client, 1).await,
        serde_json::json!({ "t": "error", "status": 400 })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_redirecting_page_sends_a_redirect_frame() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/away").await;

    request_page_run(&mut client, 1, "/away", serde_json::json!({})).await;
    assert_eq!(
        next_frame(&mut client, 1).await,
        serde_json::json!({ "t": "redirect", "location": "/target" })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_failing_page_sends_an_error_frame_with_its_status() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/broken").await;

    request_page_run(&mut client, 1, "/broken", serde_json::json!({})).await;
    assert_eq!(
        next_frame(&mut client, 1).await,
        serde_json::json!({ "t": "error", "status": 400 })
    );

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

#[tokio::test]
async fn a_malformed_message_is_answered_and_the_connection_stays_open() {
    let (addr, shutdown_tx, server) = spawn_server().await;
    let mut client = connect(addr, "/room").await;

    client.send(Message::text("not json")).await.unwrap();
    assert_eq!(
        next_json(&mut client).await,
        serde_json::json!({ "frame": { "t": "error", "status": 400 } })
    );

    request_page_run(&mut client, 1, "/room", serde_json::json!({})).await;
    assert_eq!(next_frame(&mut client, 1).await["t"], "snapshot");

    client.close(None).await.unwrap();
    shut_down(shutdown_tx, server).await;
}

/// Creates a WebSocket handshake request for `/room` with extra headers.
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
    // Calling the router directly provides no connection to upgrade.
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
