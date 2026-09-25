Request extractors and response types for Topcoat handlers.

A handler's parameters describe the request body it accepts. Its return type describes the response it sends. This module provides types for both roles.

# Reading a request body

A page or route can accept one body parameter that implements [`FromRequest`](crate::request::FromRequest). For example, [`Json`] deserializes a JSON body into your type. Add `cx: &Cx` when the handler also needs request context.

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

The context and body parameters are optional and may appear in either order. There can be only one body parameter because the body can be consumed only once. Built-in extractors reject malformed input with `400 Bad Request`. Wrap an extractor in [`Option`] to accept an absent body. Pages read bodies the same way and return a view.

# The body limit

Extractors that buffer the body reject requests above the body limit with `413 Content Too Large`. The default is 2 MiB. Register a [`BodyLimit`](crate::BodyLimit) layer to change it for the application or a path:

```rust,no_run
use topcoat::router::{BodyLimit, Router};

let router = Router::builder()
    // Allow up to 32 MiB under /upload, keep the 2 MiB default elsewhere.
    .layer(BodyLimit::max(32 * 1024 * 1024).at("/upload"))
    .build();
```

Taking [`Body`](crate::Body) directly is not limited, because the handler streams the body instead of buffering it.

Implement [`FromRequest`](crate::request::FromRequest) yourself for request parsing the built-in extractors do not cover, such as a body that is verified against a signature header before it is deserialized. Delegate the buffering to [`Bytes`](crate::request::Bytes) so the body limit stays applied.

# Returning a response

A route returns `Result<T>`. Implement [`IntoResponse`](crate::response::IntoResponse) for a synchronous conversion, or [`AsyncIntoResponse`](crate::response::AsyncIntoResponse) if building the response needs to await work. A wrapper such as [`Json`] serializes its value and sets the content type. A string or byte buffer becomes the response body directly.

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

Use [`Js`] or [`Wasm`] when serving JavaScript or WebAssembly bytes directly from a route. They set the content types required by browser module loaders.

Implement [`IntoResponse`](crate::response::IntoResponse) yourself for a type that should control its own status, headers, and body. A page sets its status and headers from inside the `view!` body instead; see the `view!` macro docs.

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
