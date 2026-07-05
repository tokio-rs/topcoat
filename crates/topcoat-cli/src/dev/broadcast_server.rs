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
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

const PORT_START: u16 = 59039;
const PORT_RANGE: u16 = 100;

const DEV_JS: &str = include_str!("dev.js");

/// An event pushed to every browser connected to the broadcast server.
#[derive(Clone, Copy)]
pub enum Event {
    /// A build is in flight.
    Rebuilding,
    /// The build failed; the previous build (if any) keeps serving.
    BuildFailed,
    /// The application process exited or could not be started.
    AppExited,
    /// A fresh build is serving; pages should reload.
    Reload,
}

impl Event {
    /// The message sent over the WebSocket; matched by `dev.js`.
    fn message(self) -> &'static str {
        match self {
            Self::Rebuilding => "rebuilding",
            Self::BuildFailed => "build-failed",
            Self::AppExited => "app-exited",
            Self::Reload => "reload",
        }
    }
}

/// Publishes [`Event`]s to every connected browser.
///
/// The bus also remembers the current status, so a page that connects
/// mid-build (or while a failure is showing) is brought up to date
/// immediately.
#[derive(Clone)]
pub struct EventBus {
    tx: Arc<broadcast::Sender<Event>>,
    status: Arc<Mutex<Option<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            tx: Arc::new(broadcast::channel(16).0),
            status: Arc::default(),
        }
    }

    /// Send `event` to every connected browser.
    pub fn publish(&self, event: Event) {
        // A reload means a fresh build is serving: nothing to report.
        *self.status.lock().unwrap() = if matches!(event, Event::Reload) {
            None
        } else {
            Some(event)
        };
        let _ = self.tx.send(event);
    }

    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    fn status(&self) -> Option<Event> {
        *self.status.lock().unwrap()
    }
}

/// Bind the broadcast server to a stable port, starting at [`PORT_START`]
/// and incrementing if occupied.
pub async fn bind() -> TcpListener {
    for port in PORT_START..=PORT_START.saturating_add(PORT_RANGE) {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)).await {
            return listener;
        }
    }
    panic!(
        "failed to bind dev server port ({PORT_START}-{})",
        PORT_START + PORT_RANGE
    );
}

/// Run the broadcast server.
///
/// Serves `/dev.js` (the client reload-and-status script) and `/ws` (the
/// WebSocket endpoint behind it). Events published on `events` are forwarded
/// to every connected client. When an application reports ready over the
/// WebSocket, its address is printed and a [`Event::Reload`] is published.
pub async fn run(listener: TcpListener, events: EventBus) {
    let app = Router::new()
        .route("/dev.js", get(serve_dev_js))
        .route("/ws", get(ws_handler))
        .with_state(events);

    let _ = axum::serve(listener, app).await;
}

async fn serve_dev_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        DEV_JS,
    )
}

async fn ws_handler(ws: WebSocketUpgrade, State(events): State<EventBus>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, events))
}

async fn handle_socket(ws: WebSocket, events: EventBus) {
    let (mut sink, mut stream) = ws.split();
    let mut rx = events.subscribe();

    // Bring a page that connects mid-build (or mid-failure) up to date.
    if let Some(event) = events.status()
        && sink
            .send(Message::Text(event.message().into()))
            .await
            .is_err()
    {
        return;
    }

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
                        events.publish(Event::Reload);
                    }
            }
            Ok(event) = rx.recv() => {
                if sink.send(Message::Text(event.message().into())).await.is_err() {
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
