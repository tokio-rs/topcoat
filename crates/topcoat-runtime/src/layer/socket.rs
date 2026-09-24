//! Renders a page or shard over a WebSocket opened at its URL.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, HeaderMap, HeaderName, HeaderValue, Method, RemoteAddr, Router, Uri,
    content::{
        ViewResponseDelivery,
        websocket::{Message, WebSocket, WebSocketUpgrade},
    },
    header,
    request::{FromRequest, IDENTITY_HEADER, Request, extensions, headers, method, uri},
    response::Response,
    router,
};

use crate::{ConnectedRender, RUNTIME_PROTOCOL, SignalValues};

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
    let target = Arc::new(ConnectionTarget::from_handshake(cx));
    upgrade
        .protocols([RUNTIME_PROTOCOL])
        .on_upgrade(move |socket| run(target, socket))
}

/// The browser's request for a new render.
///
/// A request naming a shard identity renders the shard endpoint at the
/// connection's URL. The whole message is then the endpoint's request body,
/// so it also carries the shard's arguments and signal values. Other
/// requests render the page.
#[derive(Debug, Deserialize)]
struct RenderRequest {
    /// The run id chosen by the browser. Sent back before any output so
    /// the browser can identify which render it belongs to.
    #[serde(default)]
    run: u64,
    /// The identity of the shard invocation to render.
    #[serde(default)]
    shard: Option<String>,
    /// The signal values to use for a page render.
    #[serde(default)]
    signals: SignalValues,
}

/// A render request and the message text it was parsed from.
struct Render {
    request: RenderRequest,
    /// Sent as the request body of a shard render.
    text: String,
}

/// A connection message for identifying runs, redirects, and errors.
#[derive(Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ConnectionMessage<'a> {
    /// Identifies all following output until the next run announcement.
    Run { id: u64 },
    /// Tells the browser to navigate after a render returned a redirect.
    Redirect { location: &'a str },
    /// Reports a failed render or an invalid request from the browser.
    Error { status: u16 },
}

impl ConnectionMessage<'_> {
    fn to_message(&self) -> Message {
        Message::text(serde_json::to_string(self).expect("a connection message serializes"))
    }
}

/// The router, URL, and request details used for every render on a connection.
struct ConnectionTarget {
    router: Router,
    uri: Uri,
    /// Headers from the handshake, with WebSocket headers and
    /// `Accept-Encoding` removed.
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

    /// Builds a request for the connection's URL.
    fn request(&self, method: Method, headers: HeaderMap, body: Body) -> Request {
        let mut request = Request::new(body);
        *request.method_mut() = method;
        *request.uri_mut() = self.uri.clone();
        *request.headers_mut() = headers;
        if let Some(remote) = self.remote {
            request.extensions_mut().insert(remote);
        }
        request
    }

    /// Renders the page or shard and sends its output to `out`.
    /// Stops when rendering finishes or the receiver closes.
    async fn render(&self, render: Render, out: mpsc::Sender<Message>) {
        let Render { request, text } = render;
        let run = ConnectionMessage::Run { id: request.run }.to_message();
        if out.send(run).await.is_err() {
            return;
        }

        let response = match request.shard {
            // A shard endpoint reads its arguments and signal values from
            // the body and its identity from a header, like an HTTP
            // re-render of the shard.
            Some(identity) => {
                let Ok(identity) = HeaderValue::try_from(identity) else {
                    let _ = out
                        .send(ConnectionMessage::Error { status: 400 }.to_message())
                        .await;
                    return;
                };
                let mut headers = self.headers.clone();
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                headers.insert(HeaderName::from_static(IDENTITY_HEADER), identity);
                self.router
                    .handle_with(
                        self.request(Method::POST, headers, Body::from(text)),
                        (ConnectedRender, ViewResponseDelivery::Frames),
                    )
                    .await
            }
            None => {
                self.router
                    .handle_with(
                        self.request(Method::GET, self.headers.clone(), Body::empty()),
                        (
                            ConnectedRender,
                            request.signals,
                            ViewResponseDelivery::Frames,
                        ),
                    )
                    .await
            }
        };

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

/// Handles render requests until the browser disconnects.
///
/// Each request cancels the previous render and waits for it to stop before
/// starting another. Aborting alone is not enough because a task running
/// on another worker can still send output until it yields. Waiting keeps
/// that output ahead of the next run announcement in the queue.
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
        let mut current: Option<tokio::task::JoinHandle<()>> = None;
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
                // Wait until the old render can no longer send frames.
                let _ = current.await;
            }
            let render = Render {
                request,
                text: text.as_str().to_owned(),
            };
            let target = Arc::clone(&target);
            let out = out.clone();
            current = Some(tokio::spawn(async move {
                target.render(render, out).await;
            }));
        }
        if let Some(current) = current {
            current.abort();
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
