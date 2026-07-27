use std::borrow::Cow;
use std::fmt;

use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use topcoat_core::context::Cx;
use topcoat_core::error::{Error, Result};

use crate::content::websocket::WebSocket;
use crate::error::{bad_request, method_not_allowed};
use crate::{Body, FromRequest, Response, extensions, headers, method};

/// WebSocket handshake extractor: validates the upgrade request and hands the
/// connection to a callback.
///
/// A route handler takes a `WebSocketUpgrade` parameter to become a WebSocket
/// endpoint. The extractor validates the handshake (a `GET` request with the
/// `Upgrade: websocket` headers of [RFC 6455]); the handler then calls
/// [`on_upgrade`](Self::on_upgrade) with the callback that speaks to the
/// client, and returns the response that completes the handshake. Before
/// upgrading, the builder methods can negotiate a subprotocol and bound
/// message sizes.
///
/// [RFC 6455]: https://datatracker.ietf.org/doc/html/rfc6455
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{
///         Response,
///         content::websocket::{Message, WebSocketUpgrade},
///         route,
///     },
/// };
///
/// #[route(GET "/echo")]
/// async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
///     upgrade.on_upgrade(|mut socket| async move {
///         while let Some(Ok(message)) = socket.recv().await {
///             if matches!(message, Message::Text(_) | Message::Binary(_))
///                 && socket.send(message).await.is_err()
///             {
///                 break;
///             }
///         }
///     })
/// }
/// ```
#[must_use]
pub struct WebSocketUpgrade {
    config: WebSocketConfig,
    protocols: Vec<Cow<'static, str>>,
    key: HeaderValue,
    on_upgrade: OnUpgrade,
    requested_protocols: Option<HeaderValue>,
    on_failed_upgrade: Box<dyn FnOnce(Error) + Send + 'static>,
}

impl WebSocketUpgrade {
    /// Sets the read buffer capacity. Defaults to 128 KiB.
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.config.read_buffer_size = size;
        self
    }

    /// Sets the target size of the write buffer, which batches writes to the
    /// client. Defaults to 128 KiB; `0` writes every message eagerly.
    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.config.write_buffer_size = size;
        self
    }

    /// Caps the write buffer, so writes error instead of buffering without
    /// bound when the client stops reading. Unlimited by default.
    pub fn max_write_buffer_size(mut self, max: usize) -> Self {
        self.config.max_write_buffer_size = max;
        self
    }

    /// Caps the size of a received message, closing the connection when a
    /// client exceeds it. Defaults to 64 MiB.
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config.max_message_size = Some(max);
        self
    }

    /// Caps the size of a received frame, closing the connection when a client
    /// exceeds it. Defaults to 16 MiB.
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config.max_frame_size = Some(max);
        self
    }

    /// Accepts frames a client failed to mask, instead of closing the
    /// connection as RFC 6455 mandates. Off by default; only enable it to
    /// tolerate known non-conforming clients.
    pub fn accept_unmasked_frames(mut self, accept: bool) -> Self {
        self.config.accept_unmasked_frames = accept;
        self
    }

    /// Declares the subprotocols the endpoint speaks, in order of preference.
    ///
    /// The first declared protocol that the client also requested (via the
    /// `Sec-WebSocket-Protocol` header) is selected, echoed in the handshake
    /// response, and reported by
    /// [`WebSocket::protocol`](crate::content::websocket::WebSocket::protocol).
    pub fn protocols<I>(mut self, protocols: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'static, str>>,
    {
        self.protocols = protocols.into_iter().map(Into::into).collect();
        self
    }

    /// Registers a callback for when the upgrade fails after the handshake
    /// response was already sent, for example because the client vanished.
    /// The default callback discards the error.
    pub fn on_failed_upgrade(mut self, callback: impl FnOnce(Error) + Send + 'static) -> Self {
        self.on_failed_upgrade = Box::new(callback);
        self
    }

    /// Completes the handshake, calling `callback` with the [`WebSocket`] once
    /// the client connection has switched protocols.
    ///
    /// The returned response must be the handler's return value; sending it
    /// performs the protocol switch. The callback runs on its own task, which
    /// owns the connection for as long as it runs.
    ///
    /// # Errors
    ///
    /// Returns an error if the handshake response cannot be assembled.
    pub fn on_upgrade<C, F>(self, callback: C) -> Result<Response>
    where
        C: FnOnce(WebSocket) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let protocol = negotiate_protocol(&self.protocols, self.requested_protocols.as_ref());

        let config = self.config;
        let on_upgrade = self.on_upgrade;
        let on_failed_upgrade = self.on_failed_upgrade;
        let socket_protocol = protocol.clone();
        tokio::spawn(async move {
            let upgraded = match on_upgrade.await {
                Ok(upgraded) => upgraded,
                Err(error) => {
                    on_failed_upgrade(error.into());
                    return;
                }
            };
            let stream = WebSocketStream::from_raw_socket(
                TokioIo::new(upgraded),
                Role::Server,
                Some(config),
            )
            .await;
            callback(WebSocket::new(stream, socket_protocol)).await;
        });

        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        let headers = response.headers_mut();
        headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
        headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        headers.insert(
            header::SEC_WEBSOCKET_ACCEPT,
            HeaderValue::try_from(derive_accept_key(self.key.as_bytes()))?,
        );
        if let Some(protocol) = protocol {
            headers.insert(header::SEC_WEBSOCKET_PROTOCOL, protocol);
        }
        Ok(response)
    }
}

impl fmt::Debug for WebSocketUpgrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocketUpgrade")
            .field("config", &self.config)
            .field("protocols", &self.protocols)
            .field("key", &self.key)
            .field("requested_protocols", &self.requested_protocols)
            .finish_non_exhaustive()
    }
}

/// Validates the handshake described by RFC 6455 section 4.2.1, rejecting a
/// request that is not a conforming WebSocket upgrade with a `400 Bad Request`
/// (or `405 Method Not Allowed` for a non-`GET` method).
impl FromRequest for WebSocketUpgrade {
    async fn from_request(cx: &Cx, _body: Body) -> Result<Self> {
        if method(cx) != Method::GET {
            return Err(method_not_allowed([Method::GET]).into());
        }

        let headers = headers(cx);
        if !header_contains(headers, &header::CONNECTION, "upgrade") {
            return Err(bad_request("expected `Connection: upgrade` request header").into());
        }
        if !header_eq(headers, &header::UPGRADE, "websocket") {
            return Err(bad_request("expected `Upgrade: websocket` request header").into());
        }
        if !header_eq(headers, &header::SEC_WEBSOCKET_VERSION, "13") {
            return Err(bad_request("expected `Sec-WebSocket-Version: 13` request header").into());
        }
        let key = headers
            .get(header::SEC_WEBSOCKET_KEY)
            .cloned()
            .ok_or_else(|| bad_request("missing `Sec-WebSocket-Key` request header"))?;
        if !is_valid_websocket_key(&key) {
            return Err(bad_request(
                "`Sec-WebSocket-Key` request header must be base64-encoded 16 bytes",
            )
            .into());
        }
        let requested_protocols = headers.get(header::SEC_WEBSOCKET_PROTOCOL).cloned();

        let on_upgrade = extensions(cx).get::<OnUpgrade>().cloned().ok_or_else(|| {
            bad_request("connection is not upgradable: WebSockets require an HTTP/1.1 connection")
        })?;

        Ok(Self {
            config: WebSocketConfig::default(),
            protocols: Vec::new(),
            key,
            on_upgrade,
            requested_protocols,
            on_failed_upgrade: Box::new(|_error| {}),
        })
    }
}

/// Selects the first of the endpoint's `supported` subprotocols that the
/// client `requested`, or [`None`] without an overlap.
fn negotiate_protocol(
    supported: &[Cow<'static, str>],
    requested: Option<&HeaderValue>,
) -> Option<HeaderValue> {
    let requested = requested?.to_str().ok()?;
    let protocol = supported.iter().find(|supported| {
        requested
            .split(',')
            .map(str::trim)
            .any(|candidate| candidate == supported.as_ref())
    })?;
    HeaderValue::from_str(protocol).ok()
}

/// Returns whether `key` is 16 bytes encoded as padded standard Base64.
fn is_valid_websocket_key(key: &HeaderValue) -> bool {
    let Some(encoded) = key.as_bytes().strip_suffix(b"==") else {
        return false;
    };
    encoded.len() == 22
        && encoded
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
}

/// Returns whether the comma-separated header contains `value`, compared
/// case-insensitively.
fn header_contains(headers: &HeaderMap, name: &HeaderName, value: &str) -> bool {
    headers
        .get(name)
        .and_then(|header| header.to_str().ok())
        .is_some_and(|header| {
            header
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case(value))
        })
}

/// Returns whether the header equals `value`, compared case-insensitively.
fn header_eq(headers: &HeaderMap, name: &HeaderName, value: &str) -> bool {
    headers
        .get(name)
        .and_then(|header| header.to_str().ok())
        .is_some_and(|header| header.trim().eq_ignore_ascii_case(value))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::io;
    use std::net::SocketAddr;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use http::Request;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::tungstenite;
    use topcoat_core::context::CxTestBuilder;

    use super::*;
    use crate::content::websocket::Message;
    use crate::error::{BadRequestError, MethodNotAllowedError};
    use crate::{Path, RouteFn, RouteFuture, Router, RouterService, internal_serve};

    /// The `Sec-WebSocket-Key` from RFC 6455's handshake example.
    const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";

    /// Builds a `Cx` for a request with the given method and headers.
    fn cx_with(method: Method, headers: &[(&str, &str)]) -> Cx {
        let mut builder = Request::builder().method(method).uri("/ws");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    /// The headers of a conforming handshake request.
    fn handshake_headers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("connection", "Upgrade"),
            ("upgrade", "websocket"),
            ("sec-websocket-version", "13"),
            ("sec-websocket-key", KEY),
        ]
    }

    async fn extract(cx: &Cx) -> Result<WebSocketUpgrade> {
        WebSocketUpgrade::from_request(cx, Body::empty()).await
    }

    // -- handshake validation --

    #[tokio::test]
    async fn non_get_method_is_method_not_allowed() {
        let cx = cx_with(Method::POST, &handshake_headers());
        let error = extract(&cx).await.expect_err("a POST is rejected");
        assert!(error.downcast_ref::<MethodNotAllowedError>().is_some());
    }

    #[tokio::test]
    async fn missing_or_wrong_headers_are_bad_requests() {
        for missing in 0..4 {
            let headers: Vec<_> = handshake_headers()
                .into_iter()
                .enumerate()
                .filter(|(index, _)| *index != missing)
                .map(|(_, header)| header)
                .collect();

            let cx = cx_with(Method::GET, &headers);
            let error = extract(&cx).await.expect_err("incomplete handshake");
            assert!(error.downcast_ref::<BadRequestError>().is_some());
        }

        let cx = cx_with(
            Method::GET,
            &[
                ("connection", "Upgrade"),
                ("upgrade", "websocket"),
                ("sec-websocket-version", "8"),
                ("sec-websocket-key", KEY),
            ],
        );
        let error = extract(&cx).await.expect_err("an old protocol version");
        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn invalid_websocket_keys_are_bad_requests() {
        for (case, key) in [
            ("empty", ""),
            ("invalid alphabet", "dGhlIHNhbXBsZSBub25jZQ*="),
            ("URL-safe alphabet", "----------------------=="),
            ("one padding character", "dGhlIHNhbXBsZSBub25jZQ="),
            ("no padding", "dGhlIHNhbXBsZSBub25jZQ"),
            ("excess padding", "dGhlIHNhbXBsZSBub25jZQ==="),
            ("too short", "dG9vIHNob3J0"),
            ("too long", "dGhlIHNhbXBsZSBub25jZSBsb25nZXI="),
            ("trailing whitespace", "dGhlIHNhbXBsZSBub25jZQ== "),
        ] {
            let cx = cx_with(
                Method::GET,
                &[
                    ("connection", "Upgrade"),
                    ("upgrade", "websocket"),
                    ("sec-websocket-version", "13"),
                    ("sec-websocket-key", key),
                ],
            );
            let error = extract(&cx).await.expect_err("an invalid key is rejected");
            let error = error
                .downcast_ref::<BadRequestError>()
                .expect("a bad-request error");
            assert!(
                error.description().contains("base64-encoded 16 bytes"),
                "unexpected error for {case} key {key:?}: {}",
                error.description()
            );
        }
    }

    #[test]
    fn valid_websocket_keys_have_the_rfc_6455_shape() {
        for key in [KEY, "/////////////////////w=="] {
            assert!(
                is_valid_websocket_key(&HeaderValue::from_static(key)),
                "valid key {key:?} was rejected"
            );
        }
    }

    #[tokio::test]
    async fn conforming_handshake_without_upgradable_connection_is_rejected() {
        // Header casing and the `Connection` header being a list must not
        // trip the validation; the request fails only at the upgrade handle,
        // which no test request carries.
        let cx = cx_with(
            Method::GET,
            &[
                ("connection", "keep-alive, UPGRADE"),
                ("upgrade", "WebSocket"),
                ("sec-websocket-version", "13"),
                ("sec-websocket-key", KEY),
            ],
        );
        let error = extract(&cx).await.expect_err("no upgrade handle");
        let error = error
            .downcast_ref::<BadRequestError>()
            .expect("a bad-request error");
        assert!(error.description().contains("not upgradable"));
    }

    // -- subprotocol negotiation --

    #[test]
    fn negotiates_the_first_supported_protocol() {
        let supported: Vec<Cow<'static, str>> = vec!["chat.v2".into(), "chat.v1".into()];
        let requested = HeaderValue::from_static("chat.v1, chat.v2");
        assert_eq!(
            negotiate_protocol(&supported, Some(&requested)),
            Some(HeaderValue::from_static("chat.v2"))
        );
    }

    #[test]
    fn negotiation_without_overlap_or_request_selects_nothing() {
        let supported: Vec<Cow<'static, str>> = vec!["chat.v1".into()];
        let requested = HeaderValue::from_static("log.v1");
        assert_eq!(negotiate_protocol(&supported, Some(&requested)), None);
        assert_eq!(negotiate_protocol(&supported, None), None);
        assert_eq!(negotiate_protocol(&[], Some(&requested)), None);
    }

    // -- end to end --

    /// Upgrades and echoes text and binary messages until the client closes.
    fn echo_route(cx: &Cx, body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let upgrade = WebSocketUpgrade::from_request(cx, body).await?;
            upgrade
                .protocols(["echo.v1"])
                .on_upgrade(|mut socket| async move {
                    while let Some(Ok(message)) = socket.recv().await {
                        if matches!(message, Message::Text(_) | Message::Binary(_))
                            && socket.send(message).await.is_err()
                        {
                            break;
                        }
                    }
                })
        })
    }

    fn echo_router() -> Router {
        Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Cow::Borrowed(Path::new("/ws")),
                echo_route,
            ))
            .build()
    }

    /// Serves the echo router on an ephemeral port, shutting down when the
    /// returned sender fires.
    async fn spawn_server() -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
        let service = RouterService::new(echo_router());
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(internal_serve(listener, service, async {
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

    #[tokio::test]
    async fn echoes_messages_over_a_real_connection() {
        let (addr, shutdown_tx, server) = spawn_server().await;

        let (mut client, response) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .expect("the handshake succeeds");
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        client
            .send(tungstenite::Message::text("hello"))
            .await
            .unwrap();
        assert_eq!(
            client.next().await.unwrap().unwrap(),
            tungstenite::Message::text("hello")
        );

        client
            .send(tungstenite::Message::binary(vec![1, 2, 3]))
            .await
            .unwrap();
        assert_eq!(
            client.next().await.unwrap().unwrap(),
            tungstenite::Message::binary(vec![1, 2, 3])
        );

        // The closing handshake is acknowledged, then the stream ends.
        client.close(None).await.unwrap();
        assert!(matches!(
            client.next().await,
            Some(Ok(tungstenite::Message::Close(_)))
        ));
        assert!(client.next().await.is_none());

        shut_down(shutdown_tx, server).await;
    }

    #[tokio::test]
    async fn negotiates_a_subprotocol_with_the_client() {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let (addr, shutdown_tx, server) = spawn_server().await;

        let mut request = format!("ws://{addr}/ws").into_client_request().unwrap();
        request.headers_mut().insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("chat.v1, echo.v1"),
        );
        let (mut client, response) = tokio_tungstenite::connect_async(request)
            .await
            .expect("the handshake succeeds");
        assert_eq!(
            response.headers().get(header::SEC_WEBSOCKET_PROTOCOL),
            Some(&HeaderValue::from_static("echo.v1"))
        );

        client.close(None).await.unwrap();
        shut_down(shutdown_tx, server).await;
    }

    #[tokio::test]
    async fn plain_request_to_a_websocket_route_is_a_bad_request() {
        let router = echo_router();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/ws")
            .body(Body::empty())
            .unwrap();
        let response = router.handle(request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
