WebSocket support for Topcoat routes.

A [WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API) connection starts as a normal `GET` request that asks the server to switch protocols. This module needs the `websocket` feature. It handles that handshake and the messages that follow. [`WebSocketUpgrade`] checks the request and completes the upgrade, and the resulting [`WebSocket`] exchanges [`Message`]s with the client for as long as the connection is open.

# Upgrading a request

A route becomes a WebSocket endpoint by taking a [`WebSocketUpgrade`] parameter and returning the response built by [`on_upgrade`](WebSocketUpgrade::on_upgrade). The callback passed to `on_upgrade` receives the [`WebSocket`] once the client has switched protocols. It runs on its own task, and the handler returns the handshake response right away.

```rust
use topcoat::{
    Result,
    router::{
        content::websocket::{Message, WebSocketUpgrade},
        response::Response,
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

A request that is not a valid WebSocket handshake is rejected before the callback runs. A method other than `GET` gets `405 Method Not Allowed`, and missing or malformed handshake headers get `400 Bad Request`. The extractor runs inside the handler like any other, so you can call request-scoped functions, such as a session check or `cookies(cx)`, as usual. To reject the request, return an error before calling `on_upgrade`.

# Reading the request context

The callback lives on after the handler that upgraded the connection, so it cannot borrow the `Cx` the handler was called with. Clone the `Cx` and move the clone into the callback instead. The clone reads the same app and request context.

```rust
use topcoat::{
    Result,
    context::{Cx, request_context},
    router::{
        content::websocket::{Message, WebSocketUpgrade},
        response::Response,
        route,
    },
};

struct Customer {
    name: String,
}

#[route(GET "/greet")]
async fn greet(cx: &Cx, upgrade: WebSocketUpgrade) -> Result<Response> {
    let cx = cx.clone();
    upgrade.on_upgrade(move |mut socket| async move {
        let customer: &Customer = request_context(&cx);
        let _ = socket.send(Message::text(customer.name.as_str())).await;
    })
}
```

# Messages

A [`Message`] is either application data or a protocol message. Application data is `Text`, which is always valid UTF-8, or `Binary`. Protocol messages are `Ping`, `Pong`, and `Close`. Incoming pings are answered automatically. [`recv`](WebSocket::recv) returns [`None`] once the connection is closed, so a receive loop ends cleanly. [`send`](WebSocket::send) sends a message and flushes it. To end the connection, call [`close`](WebSocket::close) to perform the closing handshake. To include a [status code](close_code) and a reason, send a [`Message::Close`] with a [`CloseFrame`] instead.

[`WebSocket`] also implements `Stream` and `Sink`. You can split it into two halves to read and write at the same time:

```rust
use futures_util::{SinkExt, StreamExt};
use topcoat::{
    Result,
    router::{
        content::websocket::{Message, WebSocketUpgrade},
        response::Response,
        route,
    },
};

#[route(GET "/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|socket| async move {
        let (mut sender, mut receiver) = socket.split();
        while let Some(Ok(message)) = receiver.next().await {
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && sender.send(message).await.is_err()
            {
                break;
            }
        }
    })
}
```

# Subprotocols and limits

[`protocols`](WebSocketUpgrade::protocols) declares the subprotocols the endpoint supports, in order of preference. The first one that the client also requested is selected, sent back in the handshake response, and returned by [`WebSocket::protocol`]. Other builder methods limit the connection. [`max_message_size`](WebSocketUpgrade::max_message_size) and [`max_frame_size`](WebSocketUpgrade::max_frame_size) protect against oversized input, and [`max_write_buffer_size`](WebSocketUpgrade::max_write_buffer_size) limits memory use when a client stops reading. See [`WebSocketUpgrade`] for the rest.
