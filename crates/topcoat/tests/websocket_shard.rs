use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Body, HeaderValue, Method, Request, Router, StatusCode, headers, to_bytes},
    runtime::{ErasedShard, RouterBuilderShardExt, shard},
    view::view,
};

struct Prefix(String);

#[shard]
async fn ordinary_shard(value: String) -> Result {
    view! { <p>(value)</p> }
}

#[shard(ws)]
async fn streaming_shard(
    cx: &Cx,
    values: tokio::sync::mpsc::Receiver<String>,
) -> impl futures_core::Stream<Item = Result> {
    let prefix = app_context::<Prefix>(cx).0.clone();
    let request_marker = headers(cx)
        .get("x-context-marker")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();

    async_stream::stream! {
        let mut values = values;
        while let Some(value) = values.recv().await {
            let text = format!("{prefix}:{request_marker}:input:{value}");
            yield view! { <p>(text)</p> };

            let text = format!("{prefix}:{request_marker}:push:{value}");
            yield view! { <p>(text)</p> };
        }
    }
}

fn shard_path(shard: ErasedShard) -> String {
    format!("/_topcoat/shards/{}", shard.id().as_str())
}

fn router() -> Router {
    Router::builder()
        .shard(ordinary_shard)
        .shard(streaming_shard)
        .app_context(Prefix("app".to_owned()))
        .build()
}

async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(topcoat::serve_until(listener, router(), async {
        let _ = shutdown_rx.await;
    }));
    (address, shutdown_tx, server)
}

async fn next_text(
    client: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> String {
    let message = tokio::time::timeout(Duration::from_secs(5), client.next())
        .await
        .expect("timed out waiting for shard output")
        .expect("the shard socket ended")
        .expect("the shard socket failed");
    message.into_text().unwrap().to_string()
}

#[tokio::test]
async fn ordinary_shards_still_use_http_post() {
    let path = shard_path(ErasedShard::from(ordinary_shard));
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(r#"["ordinary"]"#))
        .unwrap();
    let response = router().handle(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"<p>ordinary</p>");
}

#[tokio::test]
async fn websocket_shard_streams_inputs_pushes_and_request_context() {
    let path = shard_path(ErasedShard::from(streaming_shard));
    let (address, shutdown_tx, server) = spawn_server().await;

    let mut request = format!("ws://{address}{path}")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("x-context-marker", HeaderValue::from_static("request"));
    let (mut client, response) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    client
        .send(tungstenite::Message::text(r#"["one"]"#))
        .await
        .unwrap();
    assert_eq!(next_text(&mut client).await, "<p>app:request:input:one</p>");
    assert_eq!(next_text(&mut client).await, "<p>app:request:push:one</p>");

    client
        .send(tungstenite::Message::text(r#"["two"]"#))
        .await
        .unwrap();
    assert_eq!(next_text(&mut client).await, "<p>app:request:input:two</p>");
    assert_eq!(next_text(&mut client).await, "<p>app:request:push:two</p>");

    client.close(None).await.unwrap();
    shutdown_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server did not shut down")
        .unwrap()
        .unwrap();
}
