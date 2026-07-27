WebSocket support for topcoat routes.

A WebSocket starts as an ordinary `GET` request that asks the server to switch protocols. This module (behind the `websocket` feature) handles that handshake and the framing that follows: [`WebSocketUpgrade`] validates the request and completes the upgrade, and the resulting [`WebSocket`] exchanges [`Message`]s with the client for as long as the connection lives.

# Upgrading a request

A route becomes a WebSocket endpoint by taking a [`WebSocketUpgrade`] parameter and returning the response its [`on_upgrade`](WebSocketUpgrade::on_upgrade) builds. The callback passed to `on_upgrade` receives the [`WebSocket`] once the client has switched protocols, and runs on its own task; the handler itself completes immediately with the handshake response.

```rust
use topcoat::{
    Result,
    router::{
        Response,
        content::websocket::{Message, WebSocketUpgrade},
        route,
    },
};

#[route(GET "/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && socket.send(message).await.is_err()
            {
                break;
            }
        }
    })
}
```

A request that is not a conforming WebSocket handshake is rejected before the handler's callback is involved: a non-`GET` method with `405 Method Not Allowed`, and missing or malformed handshake headers with `400 Bad Request`. Because the extractor runs inside the handler like any other, request-scoped functions (a session check, `cookies(cx)`) compose with it as usual -- reject the request by returning an error before calling `on_upgrade`.

# Messages

A [`Message`] is either application data (`Text`, guaranteed UTF-8, or `Binary`) or protocol bookkeeping (`Ping`, `Pong`, and `Close`). Incoming pings are answered automatically. [`recv`](WebSocket::recv) returns [`None`] once the connection has closed, so a receive loop terminates cleanly; [`send`](WebSocket::send) delivers a message and flushes it. To end the conversation, [`close`](WebSocket::close) performs the closing handshake, or send a [`Message::Close`] carrying a [`CloseFrame`] to attach a [status code](close_code) and reason.

[`WebSocket`] also implements `Stream` and `Sink`, so the connection can be split into halves that read and write concurrently:

```rust,ignore
use futures_util::StreamExt;

let (mut sender, mut receiver) = socket.split();
```

# Subprotocols and limits

[`protocols`](WebSocketUpgrade::protocols) declares the subprotocols the endpoint speaks; the first one the client also requested is selected, echoed in the handshake response, and reported by [`WebSocket::protocol`]. Further builder methods bound the connection. [`max_message_size`](WebSocketUpgrade::max_message_size) and [`max_frame_size`](WebSocketUpgrade::max_frame_size) protect against oversized input, and [`max_write_buffer_size`](WebSocketUpgrade::max_write_buffer_size) bounds memory when a client stops reading. See [`WebSocketUpgrade`] for the rest.
