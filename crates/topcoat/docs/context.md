[`Cx`] gives handlers and components access to request data and shared application values.

Add `cx: &Cx` to a handler or component's parameters when it needs context. Topcoat supplies it automatically.

# Router request helpers

The [`router::request`](crate::router::request) module exposes small functions for reading HTTP request data from `cx`.

```rust
use topcoat::{
    context::Cx,
    router::request::{headers, method, uri},
};

fn request_summary(cx: &Cx) -> String {
    let user_agent = headers(cx)
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");

    format!("{} {} from {user_agent}", method(cx), uri(cx).path())
}
```

Use a field helper such as [`headers(cx)`](crate::router::request::headers) for one part of the request. See [`router::request`](crate::router::request) for the available helpers.

Use [`parts(cx)`](crate::router::request::parts) when you need several fields at once:

```rust
use topcoat::{context::Cx, router::request::parts};

fn cache_key(cx: &Cx) -> String {
    let parts = parts(cx);
    format!("{}:{}", parts.method, parts.uri)
}
```

Use [`extensions(cx)`](crate::router::request::extensions) for typed request values attached by a lower-level request layer or service integration:

```rust
use topcoat::{context::Cx, router::request::extensions};

struct RequestId(String);

fn request_id(cx: &Cx) -> Option<&str> {
    extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
}
```

A handler reached through a [rewrite](crate::router::error#rewrites) sees the rewritten request in the HTTP field helpers. Those helpers have `original_` counterparts returning the request as the client sent it, from [`original_parts(cx)`](crate::router::request::original_parts) down to [`original_uri(cx)`](crate::router::request::original_uri). Layers can also change the current request without a rewrite. For example, [`StripPrefixLayer`](crate::router::StripPrefixLayer) changes `uri(cx)` while `original_uri(cx)` keeps the incoming URI. The client's IP address is resolved when the request arrives and stays the same across rewrites.

# Path and query helpers

The [`path_param!`](macro@crate::router::path_param) macro and [`#[query_params]`](macro@crate::router::query_params) attribute declare typed values that you read with the [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params) functions. Topcoat parses typed path parameters and query structs lazily and memoizes them for the request.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, query_params},
    view::{View, view},
};

path_param!(post_id: uuid::Uuid, error = bad_request);

#[query_params(error = bad_request)]
struct PostQuery {
    preview: Option<bool>,
}

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    let query = query_params::<PostQuery>(cx)?;

    Ok(view! {
        <article data-preview=(query.preview.unwrap_or(false))>
            "post id: " (post_id.to_string())
        </article>
    })
}
```

Any function with `&Cx` can read these values.

# App and request context helpers

This module exposes typed context accessors:

- [`app_context::<T>(cx)`](app_context) reads a required value registered on the router with `.app_context(value)`.
- [`try_app_context::<T>(cx)`](try_app_context) reads an optional value registered on the router.
- [`request_context::<T>(cx)`](request_context) reads a required typed value attached to the current request.
- [`try_request_context::<T>(cx)`](try_request_context) reads an optional typed value attached to the current request.

```rust
use topcoat::context::{Cx, app_context};
#
# struct Database;

fn db(cx: &Cx) -> &Database {
    app_context::<Database>(cx)
}
```

Values are keyed by Rust type. The required helpers panic when the requested type was not registered, so they are best wrapped in small application-specific functions like `db(cx)`, `config(cx)`, or `current_tenant(cx)`.

Use the `try_` helpers when a value is intentionally optional on some requests:

```rust
use topcoat::context::{Cx, try_request_context};
#
# struct Customer;

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_request_context(cx)
}
```

# Registering request context

[`Cx::with`] creates a child context with an additional value. [`Cx::with_many`] adds several values at once. The child inherits other context values and shares the request's state with its parent.

```rust
use topcoat::context::{Cx, request_context};

struct Customer {
    name: String,
}

fn greet(cx: &Cx) -> String {
    let cx = cx.with(Customer {
        name: "Ada".to_owned(),
    });

    let customer: &Customer = request_context(&cx);
    format!("Hello, {}", customer.name)
}
```

Adding a type that is already present replaces it in the child scope. The parent still sees the original value. A router layer can pass a child context to the next handler to make its values available there.

# Work that outlives the handler

A [`Cx`] is a handle to state shared by everything serving one request. The router drops its own handle once the response is sent, so a streaming response body or a spawned task cannot borrow the `cx` the handler was called with. Clone the `Cx` and move the owned handle into the work instead; it reads the same app and request context.

```rust
# async fn record(name: &str) {}
use topcoat::{
    Result,
    context::{Cx, request_context},
    router::route,
};

struct Customer {
    name: String,
}

#[route(POST "/orders")]
async fn place_order(cx: &Cx) -> Result<&'static str> {
    let cx = cx.clone();
    tokio::spawn(async move {
        let customer: &Customer = request_context(&cx);
        record(&customer.name).await;
    });
    Ok("queued")
}
```

A cloned handle can still read context after the handler returns. It cannot change response headers that have already been sent. Cookie writes after the jar is sealed panic.

# Memoization

[`#[memoize]`](macro@memoize) caches a function's result for one request. Repeated calls with the same arguments share the result when they see the same request context dependencies. Use it for expensive work needed in several places. See the macro's documentation for requirements and scoping rules.

# Composing helpers

Any helper can accept `cx: &`[`Cx`], call other helpers, and return a domain-specific result:

```rust
use topcoat::{
    context::Cx,
    router::request::{headers, uri},
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
