use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use console::style;
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

const PORT_START: u16 = 59039;
const PORT_RANGE: u16 = 100;

const DEV_JS: &str = include_str!("dev.js");

/// Bind the dev server to a stable port, starting at [`PORT_START`] and
/// incrementing if occupied.
pub async fn bind() -> TcpListener {
    for port in PORT_START..=PORT_START.saturating_add(PORT_RANGE) {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)).await {
            return listener;
        }
    }
    panic!(
        "failed to bind dev server port ({PORT_START}–{})",
        PORT_START + PORT_RANGE
    );
}

/// Run the dev server.
///
/// Serves `/dev.js` (the client reload script) and `/ws` (a WebSocket endpoint).
/// When any WS client sends a ready message, prints the app listener address
/// and broadcasts `"reload"` to all other connected clients.
pub async fn run(listener: TcpListener) {
    let (tx, _) = broadcast::channel::<()>(16);
    let tx = Arc::new(tx);

    let app = Router::new()
        .route("/dev.js", get(serve_dev_js))
        .route("/ws", get(ws_handler))
        .with_state(tx);

    let _ = axum::serve(listener, app).await;
}

async fn serve_dev_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        DEV_JS,
    )
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(tx): State<Arc<broadcast::Sender<()>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(ws: WebSocket, tx: Arc<broadcast::Sender<()>>) {
    let (mut sink, mut stream) = ws.split();
    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else { break };

                if let Message::Text(text) = msg
                    && let Some(message) = ReadyMessage::parse(&text) {
                        match message {
                            ReadyMessage::Ready { addr: Some(addr) } => {
                                eprintln!(
                                    "  {} {}",
                                    style("ready on").green().bold(),
                                    style(format!("http://{addr}")).cyan()
                                );
                                eprintln!();
                            }
                            ReadyMessage::Ready { addr: None } => {
                                eprintln!("  {}", style("ready").green().bold());
                                eprintln!();
                            }
                        }
                        let _ = tx.send(());
                    }
            }
            Ok(()) = rx.recv() => {
                if sink.send(Message::Text("reload".into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

enum ReadyMessage {
    Ready { addr: Option<SocketAddr> },
}

impl ReadyMessage {
    fn parse(text: &str) -> Option<Self> {
        if text == "ready" {
            Some(Self::Ready { addr: None })
        } else {
            text.strip_prefix("ready ")
                .and_then(|addr| addr.parse().ok())
                .map(|addr| Self::Ready { addr: Some(addr) })
        }
    }
}
