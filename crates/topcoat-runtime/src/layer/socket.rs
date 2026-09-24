//! The WebSocket transport of the runtime protocol: a connection opened at
//! the page's own URL that renders the page on request.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, HeaderMap, Method, RemoteAddr, Router, Uri,
    content::{
        ViewResponseDelivery,
        websocket::{Message, WebSocket, WebSocketUpgrade},
    },
    header,
    request::{FromRequest, Request, extensions, headers, method, uri},
    response::Response,
    router,
};

use crate::{ConnectedRender, SignalValues};

/// The WebSocket subprotocol the browser requests a runtime connection with.
///
/// A handshake at a page's URL naming this protocol is answered by
/// [`RuntimeLayer`](crate::RuntimeLayer) instead of the page. Over the
/// connection, the browser sends render requests and receives the page's
/// frames.
pub const RUNTIME_PROTOCOL: &str = "topcoat-runtime";

/// Whether the request is a runtime handshake: a `GET` whose requested
/// subprotocols include the runtime's.
pub(super) fn requested(cx: &Cx) -> bool {
    *method(cx) == Method::GET && requests_runtime_protocol(headers(cx))
}

/// Whether a handshake's requested subprotocols include the runtime's.
fn requests_runtime_protocol(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|protocol| protocol.trim() == RUNTIME_PROTOCOL)
}

/// Accepts the handshake and renders the page over the connection.
pub(super) async fn accept(cx: &Cx, body: Body) -> Result<Response> {
    let upgrade = WebSocketUpgrade::from_request(cx, body).await?;
    let target = Arc::new(ConnectionTarget::from_handshake(cx));
    upgrade
        .protocols([RUNTIME_PROTOCOL])
        .on_upgrade(move |socket| run(target, socket))
}

/// A request from the browser to render the connection's page.
#[derive(Debug, Deserialize)]
struct RenderRequest {
    /// The browser's number for the run, echoed ahead of the run's output
    /// so the browser can tell the runs apart.
    #[serde(default)]
    run: u64,
    /// The document's current signal values.
    #[serde(default)]
    signals: SignalValues,
}

/// A message the connection sends besides the frames of a view response.
#[derive(Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ConnectionMessage<'a> {
    /// The output of the run with this id follows, until the next `run`.
    Run { id: u64 },
    /// The run redirected before producing content.
    Redirect { location: &'a str },
    /// The run produced no content: it failed with this status, or the
    /// browser's request could not be read.
    Error { status: u16 },
}

impl ConnectionMessage<'_> {
    fn to_message(&self) -> Message {
        Message::text(serde_json::to_string(self).expect("a connection message serializes"))
    }
}

/// The page a connection renders, with the request details every run
/// reuses from the handshake.
struct ConnectionTarget {
    router: Router,
    uri: Uri,
    /// The handshake headers without the ones that only describe the
    /// upgrade or negotiate an encoding the runtime cannot undo.
    headers: HeaderMap,
    remote: Option<RemoteAddr>,
}

impl ConnectionTarget {
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
            uri: uri(cx).clone(),
            headers,
            remote: extensions(cx).get::<RemoteAddr>().copied(),
        }
    }

    /// The logical `GET` a run dispatches for the page.
    fn request(&self) -> Request {
        let mut request = Request::new(Body::empty());
        *request.method_mut() = Method::GET;
        *request.uri_mut() = self.uri.clone();
        *request.headers_mut() = self.headers.clone();
        if let Some(remote) = self.remote {
            request.extensions_mut().insert(remote);
        }
        request
    }

    /// Renders the page once for `request`, sending the run's output to
    /// `out` until it ends or the receiver is gone.
    async fn render(&self, request: RenderRequest, out: mpsc::Sender<Message>) {
        let run = ConnectionMessage::Run { id: request.run }.to_message();
        if out.send(run).await.is_err() {
            return;
        }

        let response = self
            .router
            .handle_with(
                self.request(),
                (ConnectedRender, request.signals, ViewResponseDelivery::Frames),
            )
            .await;

        let status = response.status();
        if status.is_redirection()
            && let Some(location) = response.headers().get(header::LOCATION)
            && let Ok(location) = location.to_str()
        {
            let _ = out
                .send(ConnectionMessage::Redirect { location }.to_message())
                .await;
            return;
        }
        let has_frames = response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|value| value == "application/x-ndjson");
        if !status.is_success() || !has_frames {
            let _ = out
                .send(
                    ConnectionMessage::Error {
                        status: status.as_u16(),
                    }
                    .to_message(),
                )
                .await;
            return;
        }

        let mut frames = response.into_body().into_data_stream();
        while let Some(frame) = frames.next().await {
            let message = match frame {
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Ok(text) => Message::text(text),
                    Err(_) => ConnectionMessage::Error { status: 500 }.to_message(),
                },
                Err(_) => ConnectionMessage::Error { status: 500 }.to_message(),
            };
            if out.send(message).await.is_err() {
                return;
            }
        }
    }
}

/// Serves render requests over `socket` until the browser disconnects.
///
/// Each render request starts a run, cancelling the run before it. A run's
/// output goes out in order behind the output the earlier run already
/// queued, so the browser always sees a run's messages after its `run`
/// announcement and never interleaved with another run's.
async fn run(target: Arc<ConnectionTarget>, socket: WebSocket) {
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
        let mut current: Option<tokio::task::AbortHandle> = None;
        while let Some(Ok(message)) = stream.next().await {
            let Message::Text(text) = message else {
                continue;
            };
            let Ok(request) = serde_json::from_str::<RenderRequest>(text.as_str()) else {
                let error = ConnectionMessage::Error { status: 400 }.to_message();
                if out.send(error).await.is_err() {
                    break;
                }
                continue;
            };
            if let Some(current) = current.take() {
                current.abort();
            }
            let target = Arc::clone(&target);
            let out = out.clone();
            current = Some(
                tokio::spawn(async move { target.render(request, out).await }).abort_handle(),
            );
        }
        if let Some(current) = current {
            current.abort();
        }
        // Dropping the last sender ends the forwarder once the queue drains.
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
