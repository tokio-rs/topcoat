Read request data and build HTTP responses with the types in this module. A handler's parameters describe the input it accepts, and its return type describes the response.

# Reading a request body

A page or route handler accepts a request body through a parameter that implements [`FromRequest`](crate::request::FromRequest). For example, [`Json<T>`](Json) reads JSON into your type `T`:

```rust
# #[derive(serde::Deserialize)] struct CreateUser { name: String }
use topcoat::{
    Result,
    router::{content::Json, route},
};

#[route(POST "/api/users")]
async fn create_user(Json(input): Json<CreateUser>) -> Result<String> {
    Ok(format!("created {}", input.name))
}
```

A handler may also take `cx: &Cx` to read request context. Both parameters are optional and may appear in either order. A handler can have only one body parameter because the body can be consumed only once.

An extractor returns an error when it cannot read the request. Extractors that implement [`OptionalFromRequest`](crate::request::OptionalFromRequest) also support `Option<T>`. Each extractor defines when input counts as absent. For example, `Option<Json<T>>` returns `None` when there is no `Content-Type` header.

# The body limit

Built-in buffering extractors reject bodies larger than the request's limit with `413 Content Too Large`. Use a [`BodyLimit`](crate::BodyLimit) layer to change the limit for the whole application or a path:

```rust,no_run
use topcoat::router::{BodyLimit, Router};

let router = Router::builder()
    // Allow up to 32 MiB under /upload.
    .layer(BodyLimit::max(32 * 1024 * 1024).at("/upload"))
    .build();
```

Taking [`Body`](crate::Body) directly leaves the stream for the handler to read. The handler is responsible for enforcing a limit.

Implement [`FromRequest`](crate::request::FromRequest) for custom parsing. If your extractor buffers the body, delegate to [`Bytes`](crate::request::Bytes) through `Bytes::from_request` to enforce the request's limit.

# Returning a response

A route returns `Result<T>`, where `T` implements [`IntoResponse`](crate::response::IntoResponse) or [`AsyncIntoResponse`](crate::response::AsyncIntoResponse). For example, returning `Json<T>` serializes the value and sets `Content-Type: application/json`.

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

Implement [`IntoResponse`](crate::response::IntoResponse) when your type needs its own response format. Use [`AsyncIntoResponse`](crate::response::AsyncIntoResponse) if the conversion needs to await.

# Multipart form data

`multipart/form-data` is the request format browsers use for forms that upload files. Behind the `multipart` feature, the [`Multipart`](multipart::Multipart) extractor parses such a body and yields each form field in turn, streaming its data.

See the [`multipart`](mod@multipart) module docs for reading fields and their metadata.

# WebSockets

A WebSocket starts as an ordinary `GET` request that asks the server to switch protocols. Behind the `websocket` feature, a route serves one by taking a [`WebSocketUpgrade`](websocket::WebSocketUpgrade) parameter and returning the response its `on_upgrade` builds; the callback then exchanges messages with the client for as long as the connection lives.

See the [`websocket`](mod@websocket) module docs for the handshake, messages, subprotocols, and connection limits.

# Server-sent events

Server-sent events push a one-way stream of events from the server to the client over a plain HTTP response. Behind the `sse` feature, a route becomes such a stream by returning [`Sse`](sse::Sse) wrapping a stream of [`Event`](sse::Event)s.

See the [`sse`](mod@sse) module docs for building events, keeping idle streams alive, and resuming after a reconnect.

# Sitemaps

A sitemap lists the URLs of a site for crawlers. Behind the `sitemap` feature, a route serves one by building a [`Sitemap`](sitemap::Sitemap) entry by entry and returning it; relative entries are resolved against the base URL registered on the router.

See the [`sitemap`](mod@sitemap) module docs for serving the sitemap at `/sitemap.xml` and the fields of an entry.
