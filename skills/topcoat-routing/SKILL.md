---
name: topcoat-routing
description: Design, implement, review, and debug routing and HTTP behavior in Topcoat Rust applications. Use for module-based or explicit routes, pages, layouts, layers, parameters, request bodies, API responses, router errors, Tower middleware, multipart, server-sent events, and WebSockets.
---

# Topcoat Routing

Use `topcoat::router` and check the installed version before relying on detailed APIs. In a source checkout, read `crates/topcoat/docs/router.md`, `crates/topcoat-router/docs/module_router.md`, and the relevant content or macro guide.

## Choose registration deliberately

Prefer `module_router!()` for an application route tree:

```rust
// src/app.rs
mod api;
mod posts;

use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    topcoat::router::module_router!().discover().build()
}
```

- `module_router!()` registers module-derived pages, layouts, layers, and routes below its module.
- `.discover()` registers linked explicit-path handlers plus discovered procedures, shards, fonts, and similar items.
- Register values such as app context and asset bundles explicitly.
- `module_router!` follows compiled `mod` declarations; it does not scan files.

Each child module adds a kebab-cased segment. An `_`-prefixed module is a group with no served segment, but its layouts and layers still scope descendants. Use `path_param!` to make the enclosing segment dynamic, `path_param!(*path)` for a catch-all, and `segment!` for renames or groups. Do not combine `segment!` and `path_param!` in one module.

Adding a path string to an attribute makes that handler explicit. Register it by value or with `.discover()`. Register same-path layers manually when order matters; module discovery rejects ambiguous layer or layout ordering.

## Use the right handler

- `#[page]`: Render a view; defaults to `GET`.
- `#[layout]`: Wrap matching pages by path prefix; accept `slot: Result` and render `slot?`.
- `#[route]`: Return an `IntoResponse` value for one method, a method list, or `*`.
- `#[layer]`: Wrap request execution under a prefix. Prefer `cx` functions for application data and authorization.

```rust
use topcoat::{Result, router::{content::Json, route}};

#[route(POST "/api/widgets")]
async fn create(Json(input): Json<CreateWidget>) -> Result<Json<Widget>> {
    Ok(Json(save(input).await?))
}
```

A handler may take `cx: &Cx` and at most one body extractor, in either order. Buffered extractors such as `Json`, `Form`, `Bytes`, and `String` enforce the body limit; `Body` streams directly. Scope a `BodyLimit` layer when the default 2 MiB limit is unsuitable.

## Read parameters through `Cx`

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, query_params},
    view::view,
};

path_param!(post_id: u64, error = not_found);

#[query_params(error = bad_request)]
struct PostQuery {
    preview: Option<bool>,
}

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let id = path_param::<PostId>(cx)?;
    let query = query_params::<PostQuery>(cx)?;
    view! { <p>(id) " " (query.preview.unwrap_or(false))</p> }
}
```

Parsing is request-memoized. Let helpers read the values from `Cx` instead of threading copies through the component tree.

## Preserve HTTP semantics

Return response wrappers or tuples with a leading `StatusCode`, optional headers, and a final body. Use router errors and `RouterErrorExt` for expected failures. Unexpected errors become status 500 without exposing their messages.

When a layout catches an inner error and renders a replacement, include the intended `StatusCode`; otherwise the replacement is status 200.

For optional features:

- `multipart`: Consume fields sequentially and bound uploads.
- `sse`: Configure keep-alives and resume from `Last-Event-ID` when needed.
- `websocket`: Authenticate before `on_upgrade` and configure message and buffer limits.
- `tower`: Use `TowerRoute` for services and `TowerLayer` for middleware. A catch-all does not match its bare prefix.

Test methods, paths, layout/layer scope, parameter failures, body limits, status codes, redirects, and authorization at every independently exposed endpoint.
