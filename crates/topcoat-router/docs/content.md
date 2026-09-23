Request extractors and response types for Topcoat handlers.

A handler's signature says which request body it reads and which response it sends. This module holds the types for both sides, from a JSON body to a multipart upload, a WebSocket connection, a stream of server-sent events, or an XML sitemap.

# Reading a request body

A page or route handler can take the request context as `cx: &Cx` and one request body parameter. The body parameter can be any type that implements [`FromRequest`](crate::request::FromRequest). [`Json`] and [`Form`] parse the body into a type of your own. [`Bytes`](crate::request::Bytes) and [`String`] give you the raw body, and [`Body`](crate::Body) gives you a stream to read yourself.

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

Both parameters are optional and can come in either order. Only one body parameter is allowed, because the body is a stream that can be read only once. When an extractor cannot parse the body, the request gets `400 Bad Request`. Some extractors, such as [`Json`] and [`Form`], can be wrapped in [`Option`] to also accept a request without a value of that kind. Pages read bodies in the same way, but render a view instead of returning a response.

# The body limit

Extractors that buffer the body read at most the request's body limit. A larger body is rejected with `413 Content Too Large`, so a client cannot use up the server's memory. The limit is 2 MiB by default. Register a [`BodyLimit`](crate::BodyLimit) layer to change it, either for the whole application or for the routes under a path:

```rust,no_run
use topcoat::router::{BodyLimit, Router};

let router = Router::builder()
    // Allow up to 32 MiB under /upload, keep the 2 MiB default elsewhere.
    .layer(BodyLimit::max(32 * 1024 * 1024).at("/upload"))
    .build();
```

Taking [`Body`](crate::Body) directly has no limit, because the handler reads the stream itself instead of buffering it.

To parse a request in a way the built-in extractors do not cover, implement [`FromRequest`](crate::request::FromRequest) yourself. One example is a body that must be checked against a signature header before it is parsed. Read the body through [`Bytes`](crate::request::Bytes) inside your implementation, so that the body limit still applies.

# Returning a response

A route returns `Result<T>`, where `T` implements [`IntoResponse`](crate::response::IntoResponse). If building the response has to await something first, as a view does, `T` can implement [`AsyncIntoResponse`](crate::response::AsyncIntoResponse) instead. The wrappers above also work as return values. They serialize the value and set the matching `Content-Type`. A string or a byte buffer becomes the body unchanged.

A tuple builds a response from several parts. The last element is the body. A [`StatusCode`](crate::StatusCode) at the start sets the status, and the elements in between add headers or extensions:

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

[`Js`] and [`Wasm`] can only be used as responses. They exist for the two media types that browsers check strictly. A browser does not run a `<script type="module">` unless it arrives as JavaScript, and `WebAssembly.compileStreaming` rejects anything that is not exactly `application/wasm`. Use them when a route serves a script or a WebAssembly module by hand instead of through the asset bundle.

Implement [`IntoResponse`](crate::response::IntoResponse) yourself for a type that should control its own status, headers, and body. A page sets its status and headers from inside its `view!` body instead. See the `view!` macro docs.

# Multipart form data

`multipart/form-data` is the format browsers use for forms that upload files. With the `multipart` feature, the [`Multipart`](multipart::Multipart) extractor parses such a body and yields its form fields one by one, streaming the data of each.

See the [`multipart`](mod@multipart) module docs for reading fields and their metadata.

# WebSockets

A WebSocket connection starts as a normal `GET` request that asks the server to switch protocols. With the `websocket` feature, a route serves one by taking a [`WebSocketUpgrade`](websocket::WebSocketUpgrade) parameter and returning the response built by its `on_upgrade` method. The callback passed to `on_upgrade` then exchanges messages with the client for as long as the connection is open.

See the [`websocket`](mod@websocket) module docs for the handshake, messages, subprotocols, and connection limits.

# Server-sent events

Server-sent events send a one-way stream of events from the server to the client over a normal HTTP response. With the `sse` feature, a route returns such a stream as an [`Sse`](sse::Sse) that wraps a stream of [`Event`](sse::Event)s.

See the [`sse`](mod@sse) module docs for building events, keeping idle streams open, and resuming after a reconnect.

# Sitemaps

A sitemap lists the URLs of a site for search engine crawlers. With the `sitemap` feature, a route builds a [`Sitemap`](sitemap::Sitemap) one entry at a time and returns it. Relative entries are resolved against the base URL registered on the router.

See the [`sitemap`](mod@sitemap) module docs for serving the sitemap at `/sitemap.xml` and for the fields of an entry.
