WebSocket connections for Topcoat routes.

A WebSocket lets a client and server exchange messages over a persistent connection. Enable the `websocket` feature to accept upgrade requests with [`WebSocketUpgrade`] and exchange messages through [`WebSocket`].

# Upgrading a request

Accept a [`WebSocketUpgrade`] parameter and return the response from [`on_upgrade`](WebSocketUpgrade::on_upgrade). Its callback receives the upgraded [`WebSocket`] and runs in a separate task. The handler returns the handshake response without waiting for that task to finish.

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

The extractor rejects non-`GET` requests with `405 Method Not Allowed` and invalid handshake headers with `400 Bad Request`. Perform application checks, such as authentication, before calling `on_upgrade`. Return an error to reject the connection.

# Reading the request context

The callback outlives the handler that upgraded the connection, so it cannot borrow the `Cx` the handler was called with. Clone the `Cx` and move the owned handle into the callback instead; it reads the same app and request context.

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

A [`Message`] is either application data (`Text`, guaranteed UTF-8, or `Binary`) or protocol bookkeeping (`Ping`, `Pong`, and `Close`). Incoming pings are answered automatically. [`recv`](WebSocket::recv) returns [`None`] once the connection has closed, so a receive loop terminates cleanly; [`send`](WebSocket::send) delivers a message and flushes it. To end the conversation, [`close`](WebSocket::close) performs the closing handshake, or send a [`Message::Close`] carrying a [`CloseFrame`] to attach a [status code](close_code) and reason.

[`WebSocket`] also implements `Stream` and `Sink`, so the connection can be split into halves that read and write concurrently:

```rust,ignore
use futures_util::StreamExt;

let (mut sender, mut receiver) = socket.split();
```

# Subprotocols and limits

Pass supported subprotocols to [`protocols`](WebSocketUpgrade::protocols) in preference order. Topcoat selects the first one also requested by the client. Read the result with [`WebSocket::protocol`]. Use the [`WebSocketUpgrade`] builder to set message, frame, and write buffer limits.
