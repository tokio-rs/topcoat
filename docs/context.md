# Request context (`Cx`)

`Cx` is Topcoat's request context. Pages, layouts, components, and routes can take it as an optional parameter when they need request-scoped information.

Add `cx: &Cx` to the function signature when needed; leave it out when the function does not need request context. Topcoat passes it automatically when the parameter is present.

## Router request helpers

The `topcoat::router` module exposes small functions for reading HTTP request data from `cx`.

```rust
use topcoat::{
    context::Cx,
    router::{headers, method, uri},
};

fn request_summary(cx: &Cx) -> String {
    let user_agent = headers(cx)
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");

    format!("{} {} from {user_agent}", method(cx), uri(cx).path())
}
```

Available request helpers:

- `parts(cx)` returns the current request's `http::request::Parts`.
- `method(cx)` returns the HTTP method.
- `uri(cx)` returns the request URI.
- `version(cx)` returns the HTTP version.
- `headers(cx)` returns the request headers.
- `extensions(cx)` returns request extensions.

Use `parts(cx)` when you need several fields at once:

```rust
use topcoat::{context::Cx, router::parts};

fn cache_key(cx: &Cx) -> String {
    let parts = parts(cx);
    format!("{}:{}", parts.method, parts.uri)
}
```

Use `extensions(cx)` for typed request values inserted by lower-level Axum middleware:

```rust
use topcoat::{context::Cx, router::extensions};

struct RequestId(String);

fn request_id(cx: &Cx) -> Option<&str> {
    extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
}
```

## Path and query helpers

Path and query parameter macros generate `of(cx)` helpers. They parse lazily and memoize the parsed value for the request.

```rust,ignore
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, query_params},
    view::view,
};

#[path_param]
struct PostId(uuid::Uuid);

#[query_params]
struct PostQuery {
    preview: Option<bool>,
}

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let post_id = PostId::of(cx).unwrap();
    let query = PostQuery::of(cx).unwrap();

    view! {
        <article data-preview=(query.preview.unwrap_or(false))>
            "post id: " (post_id.to_string())
        </article>
    }
}
```

See [Path and query params](./path_and_query_params.md) for the exact return types and parsing rules.

## App and request state helpers

The `topcoat::context` module exposes typed state accessors:

- `app_state::<T>(cx)` reads state registered on the router with `.app_state(value)`.
- `request_state::<T>(cx)` reads typed state attached to the current request.

```rust
use topcoat::context::{Cx, app_state};

struct Database;

fn db(cx: &Cx) -> &Database {
    app_state::<Database>(cx)
}
```

State is keyed by Rust type. Asking for a type that was not registered panics, so these helpers are best wrapped in small application-specific functions like `db(cx)`, `config(cx)`, or `current_tenant(cx)`.

## Extractor escape hatch

Prefer Topcoat's dedicated helpers when they exist. If you need to interoperate with an Axum extractor that implements `FromRequestParts`, use `router::extract`.

```rust
use axum::extract::Query;
use serde::Deserialize;
use topcoat::{context::Cx, router::extract};

#[derive(Deserialize)]
struct Pagination {
    page: usize,
}

async fn page_number(cx: &Cx) -> Option<usize> {
    let Query(pagination): Query<Pagination> = extract::<_, ()>(cx).await.ok()?;
    Some(pagination.page)
}
```

## Composing helpers

Any helper can accept `cx: &Cx`, call other helpers, and return a domain-specific result:

```rust
use topcoat::{
    context::Cx,
    router::{headers, uri},
};

fn locale(cx: &Cx) -> &str {
    headers(cx)
        .get("accept-language")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .unwrap_or("en")
}

fn canonical_url(cx: &Cx) -> String {
    format!("https://example.com{}", uri(cx).path())
}
```

That keeps pages, layouts, components, and routes focused on rendering or responding while shared request reads stay in ordinary Rust functions.
