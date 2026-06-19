# Request and response bodies

Topcoat route and page handlers can receive the request context with `cx: &Cx`. They can also receive one request body parameter. That parameter can be any type that implements `FromRequest`. API routes additionally return `Result<T>` where `T: IntoResponse`.

## JSON

Use `Json<T>` for JSON request bodies and JSON responses:

```rust
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    router::{Json, route},
};

#[derive(Deserialize, Serialize)]
struct CreateUser {
    name: String,
}

#[route(POST "/api/users")]
async fn create_user(Json(input): Json<CreateUser>) -> Result<Json<CreateUser>> {
    Ok(Json(CreateUser {
        name: input.name,
    }))
}
```

`Json<T>` requires `T: DeserializeOwned` for requests and `T: Serialize` for responses. Requests must use `Content-Type: application/json` or an `application/*+json` content type.

## Forms

Use `Form<T>` for `application/x-www-form-urlencoded` bodies:

```rust
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    router::{Form, Json, route},
};

#[derive(Deserialize, Serialize)]
struct Search {
    q: String,
}

#[route(POST "/api/search")]
async fn search(Form(input): Form<Search>) -> Result<Json<Search>> {
    Ok(Json(input))
}
```

For `GET` and `HEAD` requests, `Form<T>` reads the query string instead of the request body. For other methods, it expects `Content-Type: application/x-www-form-urlencoded`.

`RawForm` uses the same source selection (query string for `GET` and `HEAD`, body otherwise) but yields the raw `Bytes` without deserializing them:

```rust
use topcoat::{
    Result,
    router::{RawForm, route},
};

#[route(POST "/api/raw-form")]
async fn raw_form(RawForm(bytes): RawForm) -> Result<String> {
    Ok(format!("received {} bytes", bytes.len()))
}
```

## Multipart form data

Enable the `multipart` feature to extract `multipart/form-data` requests (commonly used for file uploads) with `Multipart`:

```rust
use topcoat::{
    Result,
    router::{Multipart, route},
};

#[route(POST "/api/files")]
async fn files(mut multipart: Multipart) -> Result<String> {
    let mut total = 0;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().map(str::to_owned);
        let data = field.bytes().await?;

        println!("field {name:?}: {} bytes", data.len());
        total += data.len();
    }

    Ok(format!("received {total} bytes"))
}
```

Each `Field` exposes `name`, `file_name`, `content_type`, and `headers`, along with `bytes`, `text`, and `chunk` for reading the data. A `Field` also implements `Stream`, so its chunks can be consumed with the usual stream combinators.

## Optional request parsing

Wrap `Json<T>`, `Form<T>`, or `Multipart` in `Option` to make the body optional. The request parser yields `None` when the request carries no body of that kind, and still reports an error when a body is present but malformed:

```rust
use serde::Deserialize;
use topcoat::{
    Result,
    router::{Json, route},
};

#[derive(Deserialize)]
struct Filter {
    tag: String,
}

#[route(POST "/api/items")]
async fn items(filter: Option<Json<Filter>>) -> Result<String> {
    match filter {
        Some(Json(filter)) => Ok(format!("filtering by {}", filter.tag)),
        None => Ok("no filter".to_string()),
    }
}
```

For `Json` and `Multipart`, `None` means the request had no `Content-Type` header. For `Form`, a `GET` or `HEAD` request yields `None` only when there is no query string. Implement `OptionalFromRequest` to give a custom extractor the same `Option<_>` behavior.

## Handler signatures

A page or route can take `cx: &Cx`, one request parameter, both, or neither:

```rust
use topcoat::{
    context::Cx,
    Result,
    router::{Json, route},
};

#[route(POST "/api/items")]
async fn with_context(cx: &Cx, Json(input): Json<CreateUser>) -> Result<Json<CreateUser>> {
    let _ = cx;
    Ok(Json(input))
}
```

The order does not matter. The request parameter may use a normal identifier or a destructuring pattern such as `Json(input): Json<CreateUser>`.

There can only be one request body parameter because the body is a stream and can only be consumed once.

Pages use the same `FromRequest` parsing, but they still return rendered views:

```rust
use topcoat::{
    Result,
    router::{Form, page},
    view::view,
};

#[page("/contact")]
async fn contact(Form(input): Form<Search>) -> Result {
    view! {
        <main>
            "searching for " (input.q)
        </main>
    }
}
```

## Response conversion

Route functions return `Result<T>` where `T: IntoResponse`:

```rust
#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

The macro calls `IntoResponse::into_response` on the successful value. Many common response shapes work through Axum's response support, including strings, status codes, byte buffers, and `(headers, body)` tuples.

For JSON, return `Json<T>`:

```rust
#[route(GET "/api/user")]
async fn user() -> Result<Json<CreateUser>> {
    Ok(Json(CreateUser {
        name: "Ada".to_string(),
    }))
}
```

A raw `Result<CreateUser>` is not automatically serialized as JSON. It only works if `CreateUser` itself implements `IntoResponse`.

## Raw bodies

Several extractors hand the handler the request body without imposing a content type:

- `Body` — the raw, unbuffered body stream, for handlers that read or forward it themselves.
- `Bytes` — the fully buffered body as `Bytes`.
- `String` — the fully buffered body decoded as UTF-8 (a non-UTF-8 body is rejected with `400`).
- `BytesMut` — the fully buffered body in a mutable buffer.

```rust
use topcoat::{
    Result,
    router::{Bytes, route},
};

#[route(POST "/api/upload")]
async fn upload(body: Bytes) -> Result<String> {
    Ok(format!("received {} bytes", body.len()))
}
```

Use `Body` directly when you need the unbuffered stream:

```rust
use topcoat::{
    Result,
    router::{Body, route},
};

#[route(POST "/api/stream")]
async fn stream(body: Body) -> Result<&'static str> {
    let _ = body;
    Ok("received")
}
```

`Bytes`, `String`, `Vec<u8>`, and `&'static str` also work as return types, so a handler can echo raw bytes or text straight back as a response.

## Custom request parsing

Implement `FromRequest` when a handler needs request-specific parsing that is not covered by `Json<T>` or `Form<T>`.

```rust
use axum::body::to_bytes;
use serde::de::DeserializeOwned;
use topcoat::{
    context::Cx,
    router::{Body, FromRequest, bad_request, headers},
    Result,
};

struct SignedJson<T>(T);

fn verify_signature(_signature: &str, _bytes: &[u8]) -> Result<()> {
    Ok(())
}

impl<T> FromRequest for SignedJson<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let signature = headers(cx)
            .get("x-signature")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| bad_request("missing x-signature header"))?;

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        verify_signature(signature, &bytes)?;

        Ok(Self(serde_json::from_slice(&bytes)?))
    }
}
```

Once the type implements `FromRequest`, use it like the built-in wrappers:

```rust
#[route(POST "/api/signed")]
async fn signed(SignedJson(input): SignedJson<CreateUser>) -> Result<Json<CreateUser>> {
    Ok(Json(input))
}
```

## Custom responses

Implement `IntoResponse` when a domain-specific response type should control its status, headers, or body.

```rust
use topcoat::{
    Result,
    router::{Body, IntoResponse, Response, route},
};

struct Csv(String);

impl IntoResponse for Csv {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", "text/csv; charset=utf-8")
            .body(Body::from(self.0))?)
    }
}

#[route(GET "/api/report.csv")]
async fn report() -> Result<Csv> {
    Ok(Csv("name,total\nAda,42\n".to_string()))
}
```

If your type already implements `axum::response::IntoResponse`, Topcoat can use that automatically through its blanket `IntoResponse` implementation.
