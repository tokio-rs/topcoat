Request extractors and response types for Topcoat handlers.

A handler declares the request body it accepts and the response it sends through its own signature. This module holds the types that fill those two roles, from a JSON body to a multipart upload, a WebSocket connection, or a stream of server-sent events.

# Reading a request body

A page or route handler can take the request context as `cx: &Cx` and, alongside it, a single request body parameter. That parameter can be any type that implements [`FromRequest`](crate::FromRequest). [`Json`] and [`Form`] deserialize the body into a type of your own, while [`Bytes`](crate::Bytes) and [`String`] hand it over unparsed and [`Body`](crate::Body) leaves it as a stream to read yourself.

```rust
# #[derive(serde::Deserialize)] struct CreateUser { name: String }
use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, route},
};

#[route(POST "/api/users")]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<String> {
    let _ = cx;
    Ok(format!("created {}", input.name))
}
```

The context and the body parameter are both optional and may appear in either order, but there can be at most one body parameter, because the body is a stream that can only be consumed once. A body an extractor cannot parse is rejected with `400 Bad Request`; wrap the extractor in [`Option`] to accept a request that carries no body at all. Pages read bodies the same way, but render a view instead of returning a response value.

# The body limit

Extractors that buffer the body read at most the request's body limit and reject a larger body with `413 Content Too Large`, so a client cannot exhaust the server's memory. The limit defaults to 2 MiB; register the [`BodyLimit`](crate::BodyLimit) layer to change it, for the whole application or for the routes under a path:

```rust
use topcoat::router::{BodyLimit, Router};

let router = Router::builder()
    // Allow up to 32 MiB under /upload, keep the 2 MiB default elsewhere.
    .layer(BodyLimit::max(32 * 1024 * 1024).at("/upload"))
    .build();
```

Taking [`Body`](crate::Body) directly is not limited, because the handler streams the body instead of buffering it.

Implement [`FromRequest`](crate::FromRequest) yourself for request parsing the built-in extractors do not cover, such as a body that is verified against a signature header before it is deserialized. Delegate the buffering to [`Bytes`](crate::Bytes) so the body limit stays applied.

# Returning a response

A route returns `Result<T>` for any `T` that implements [`IntoResponse`](crate::IntoResponse). The same wrappers work in return position, where they serialize the value and set the matching `Content-Type`; a string or byte buffer becomes the body as is.

A tuple builds a response from several parts. The last element is the body, a leading [`StatusCode`](crate::StatusCode) sets the status, and the elements in between attach headers or extensions:

```rust
# #[derive(serde::Serialize)] struct User { name: String }
use topcoat::{
    Result,
    router::{StatusCode, content::Json, route},
};

#[route(POST "/api/users")]
async fn create_user() -> Result<(StatusCode, Json<User>)> {
    let user = User {
        name: "Ada".to_string(),
    };
    Ok((StatusCode::CREATED, Json(user)))
}
```

Implement [`IntoResponse`](crate::IntoResponse) yourself for a type that should control its own status, headers, and body. A page sets its status and headers from inside the `view!` body instead; see the `view!` macro docs.

# Multipart form data

`multipart/form-data` is the request format browsers use for forms that upload files. Behind the `multipart` feature, the [`Multipart`](multipart::Multipart) extractor parses such a body and yields each form field in turn, streaming its data.

See the [`multipart`](mod@multipart) module docs for reading fields and their metadata.

# WebSockets

A WebSocket starts as an ordinary `GET` request that asks the server to switch protocols. Behind the `websocket` feature, a route serves one by taking a [`WebSocketUpgrade`](websocket::WebSocketUpgrade) parameter and returning the response its `on_upgrade` builds; the callback then exchanges messages with the client for as long as the connection lives.

See the [`websocket`](mod@websocket) module docs for the handshake, messages, subprotocols, and connection limits.

# Server-sent events

Server-sent events push a one-way stream of events from the server to the client over a plain HTTP response. Behind the `sse` feature, a route becomes such a stream by returning [`Sse`](sse::Sse) wrapping a stream of [`Event`](sse::Event)s.

See the [`sse`](mod@sse) module docs for building events, keeping idle streams alive, and resuming after a reconnect.
