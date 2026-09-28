//! Renders pages and shards over one WebSocket per document.

use std::{collections::HashMap, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::JoinHandle};
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, HeaderMap, HeaderName, HeaderValue, Method, RemoteAddr, Router, Uri,
    content::{
        ViewResponseDelivery,
        websocket::{Message, WebSocket, WebSocketUpgrade},
    },
    header,
    request::{FromRequest, IDENTITY_HEADER, Request, extensions, headers, method},
    response::Response,
    router,
};

use crate::{ConnectedRender, RUNTIME_HEADER, RUNTIME_PROTOCOL};

/// Checks for a `GET` that requests the runtime WebSocket subprotocol.
pub(super) fn requested(cx: &Cx) -> bool {
    *method(cx) == Method::GET && requests_runtime_protocol(headers(cx))
}

/// Checks whether the headers request the runtime subprotocol.
fn requests_runtime_protocol(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|protocol| protocol.trim() == RUNTIME_PROTOCOL)
}

/// Opens the WebSocket and starts handling render requests.
pub(super) async fn accept(cx: &Cx, body: Body) -> Result<Response> {
    let upgrade = WebSocketUpgrade::from_request(cx, body).await?;
    let connection = Arc::new(Connection::from_handshake(cx));
    upgrade
        .protocols([RUNTIME_PROTOCOL])
        .on_upgrade(move |socket| run(connection, socket))
}

/// A message from the browser.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ClientMessage {
    /// Stops a run. Output the run already sent may still arrive.
    Stop { stop: u64 },
    /// Starts a new run alongside any others.
    Run(RunRequest),
}

/// The browser's request for a connected render.
///
/// The fields describe an HTTP request, the same one the browser would
/// send to re-render the content without a connection.
#[derive(Debug, Deserialize)]
struct RunRequest {
    /// The run id chosen by the browser. Every message of the run's output
    /// carries it.
    run: u64,
    method: String,
    /// An absolute path, with an optional query.
    path: String,
    /// Headers added to the ones from the handshake. See
    /// [`allowed_header`].
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: String,
}

/// A frame reporting a redirect or error instead of rendered output.
#[derive(Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ConnectionFrame<'a> {
    /// Tells the browser to navigate after a render returned a redirect.
    Redirect { location: &'a str },
    /// Reports a failed render or an invalid request from the browser.
    Error { status: u16 },
}

impl ConnectionFrame<'_> {
    fn to_message(&self, run: Option<u64>) -> Message {
        let frame = serde_json::to_string(self).expect("a connection frame serializes");
        envelope(run, &frame)
    }
}

/// Wraps one JSON frame in a message naming its run.
///
/// Runs share the connection, so their output interleaves. The browser
/// routes each frame by its run and drops frames of runs it has stopped.
/// A message without a run answers a message that could not be parsed.
fn envelope(run: Option<u64>, frame: &str) -> Message {
    let frame = frame.trim_end();
    Message::text(match run {
        Some(run) => format!("{{\"run\":{run},\"frame\":{frame}}}"),
        None => format!("{{\"frame\":{frame}}}"),
    })
}

/// Checks whether a run request may set a header: the content type, and
/// the headers that mark shard and page re-renders. Everything else comes
/// from the handshake, so a run cannot change the credentials, origin, or
/// host the handshake established.
fn allowed_header(name: &HeaderName) -> bool {
    *name == header::CONTENT_TYPE || name == IDENTITY_HEADER || *name == RUNTIME_HEADER
}

/// The router and request details shared by every render on a connection.
struct Connection {
    router: Router,
    /// Headers from the handshake, with WebSocket headers and
    /// `Accept-Encoding` removed.
    headers: HeaderMap,
    remote: Option<RemoteAddr>,
}

impl Connection {
    fn from_handshake(cx: &Cx) -> Self {
        let mut headers = headers(cx).clone();
        for name in [
            header::CONNECTION,
            header::UPGRADE,
            header::SEC_WEBSOCKET_KEY,
            header::SEC_WEBSOCKET_VERSION,
            header::SEC_WEBSOCKET_PROTOCOL,
            header::SEC_WEBSOCKET_EXTENSIONS,
            header::ACCEPT_ENCODING,
        ] {
            headers.remove(name);
        }
        Self {
            router: router(cx),
            headers,
            remote: extensions(cx).get::<RemoteAddr>().copied(),
        }
    }

    /// Builds the HTTP request a run describes. Returns `None` for an
    /// invalid method, header, or path, a path that is not absolute, or a
    /// header that [`allowed_header`] rejects.
    fn request(&self, run: RunRequest) -> Option<Request> {
        let method = Method::from_bytes(run.method.as_bytes()).ok()?;
        let uri = Uri::try_from(run.path).ok()?;
        if uri.scheme().is_some() || !uri.path().starts_with('/') {
            return None;
        }
        let mut headers = self.headers.clone();
        for (name, value) in run.headers {
            let name = HeaderName::try_from(name).ok()?;
            if !allowed_header(&name) {
                return None;
            }
            headers.insert(name, HeaderValue::try_from(value).ok()?);
        }

        let mut request = Request::new(Body::from(run.body));
        *request.method_mut() = method;
        *request.uri_mut() = uri;
        *request.headers_mut() = headers;
        if let Some(remote) = self.remote {
            request.extensions_mut().insert(remote);
        }
        Some(request)
    }

    /// Dispatches the run's request as a connected render and sends its
    /// output to `out`. Stops when rendering finishes or the receiver
    /// closes.
    async fn render(&self, run: RunRequest, out: mpsc::Sender<Message>) {
        let id = Some(run.run);
        let Some(request) = self.request(run) else {
            let error = ConnectionFrame::Error { status: 400 };
            let _ = out.send(error.to_message(id)).await;
            return;
        };
        let response = self
            .router
            .handle_with(request, (ConnectedRender, ViewResponseDelivery::Frames))
            .await;

        let status = response.status();
        if status.is_redirection()
            && let Some(location) = response.headers().get(header::LOCATION)
            && let Ok(location) = location.to_str()
        {
            let redirect = ConnectionFrame::Redirect { location };
            let _ = out.send(redirect.to_message(id)).await;
            return;
        }
        let has_frames = response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|value| value == "application/x-ndjson");
        if !status.is_success() || !has_frames {
            let error = ConnectionFrame::Error {
                status: status.as_u16(),
            };
            let _ = out.send(error.to_message(id)).await;
            return;
        }

        let mut frames = response.into_body().into_data_stream();
        while let Some(frame) = frames.next().await {
            let message = match frame.as_deref().map(std::str::from_utf8) {
                Ok(Ok(text)) => envelope(id, text),
                _ => ConnectionFrame::Error { status: 500 }.to_message(id),
            };
            if out.send(message).await.is_err() {
                return;
            }
        }
    }
}

/// Handles run requests until the browser disconnects.
///
/// Runs proceed side by side until they finish, the browser stops them, or
/// the connection closes. A stopped run can still send output until its
/// task next yields; the browser drops that output by its run id.
async fn run(connection: Arc<Connection>, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (out, mut queue) = mpsc::channel::<Message>(16);

    let forward = async move {
        while let Some(message) = queue.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    };

    let receive = async move {
        let mut runs = HashMap::<u64, JoinHandle<()>>::new();
        while let Some(Ok(message)) = stream.next().await {
            let Message::Text(text) = message else {
                continue;
            };
            let request = match serde_json::from_str::<ClientMessage>(text.as_str()) {
                Ok(ClientMessage::Run(request)) => request,
                Ok(ClientMessage::Stop { stop }) => {
                    if let Some(run) = runs.remove(&stop) {
                        run.abort();
                    }
                    continue;
                }
                Err(_) => {
                    let error = ConnectionFrame::Error { status: 400 }.to_message(None);
                    if out.send(error).await.is_err() {
                        break;
                    }
                    continue;
                }
            };
            runs.retain(|_, run| !run.is_finished());
            let id = request.run;
            let connection = Arc::clone(&connection);
            let out = out.clone();
            let handle = tokio::spawn(async move {
                connection.render(request, out).await;
            });
            // A reused id replaces the run that had it.
            if let Some(previous) = runs.insert(id, handle) {
                previous.abort();
            }
        }
        for run in runs.values() {
            run.abort();
        }
        // The forwarder finishes after all senders close and queued messages are sent.
    };

    futures_util::future::join(forward, receive).await;
}

#[cfg(test)]
mod tests {
    use topcoat_router::HeaderValue;

    use super::*;

    fn headers_with_protocols(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static(value),
        );
        headers
    }

    #[test]
    fn the_runtime_protocol_is_recognized_among_others() {
        assert!(requests_runtime_protocol(&headers_with_protocols(
            "topcoat-runtime"
        )));
        assert!(requests_runtime_protocol(&headers_with_protocols(
            "chat.v1, topcoat-runtime"
        )));
        assert!(!requests_runtime_protocol(&headers_with_protocols(
            "topcoat-runtime-v2"
        )));
        assert!(!requests_runtime_protocol(&HeaderMap::new()));
    }
}
