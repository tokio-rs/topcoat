[`Cx`] is Topcoat's request context. Pages, layouts, components, and routes can take it as an optional parameter when they need request-scoped information.

Add `cx: &Cx` to the function signature when needed; leave it out when the function does not need request context. Topcoat passes it automatically when the parameter is present.

# Router request helpers

The [`router`](crate::router) module exposes small functions for reading HTTP request data from `cx`.

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

- [`parts(cx)`](crate::router::parts) returns the current request's `http::request::Parts`.
- [`method(cx)`](crate::router::method) returns the HTTP method.
- [`uri(cx)`](crate::router::uri) returns the request URI.
- [`version(cx)`](crate::router::version) returns the HTTP version.
- [`headers(cx)`](crate::router::headers) returns the request headers.
- [`content_type(cx)`](crate::router::content_type) returns the request `Content-Type`.
- [`extensions(cx)`](crate::router::extensions) returns request extensions.

Use [`parts(cx)`](crate::router::parts) when you need several fields at once:

```rust
use topcoat::{context::Cx, router::parts};

fn cache_key(cx: &Cx) -> String {
    let parts = parts(cx);
    format!("{}:{}", parts.method, parts.uri)
}
```

Use [`extensions(cx)`](crate::router::extensions) for typed request values attached by a lower-level request layer or service integration:

```rust
use topcoat::{context::Cx, router::extensions};

struct RequestId(String);

fn request_id(cx: &Cx) -> Option<&str> {
    extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
}
```

# Path and query helpers

The attribute macros [`#[path_param]`](macro@crate::router::path_param) and [`#[query_params]`](macro@crate::router::query_params) declare typed structs that you read with the [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params) functions. They parse lazily and memoize the parsed value for the request.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, query_params},
    view::view,
};

#[path_param(error = bad_request)]
struct PostId(uuid::Uuid);

#[query_params(error = bad_request)]
struct PostQuery {
    preview: Option<bool>,
}

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx)?;
    let query = query_params::<PostQuery>(cx)?;

    view! {
        <article data-preview=(query.preview.unwrap_or(false))>
            "post id: " (post_id.to_string())
        </article>
    }
}
```

This means your params are available anywhere you have access to a `cx`. See the attribute macro's documentation for more details.

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

# Memoization

[`#[memoize]`](macro@memoize) caches a `cx`-taking function's result for the duration of a request, keyed by its arguments. Wrap the request helpers above with it so that repeated calls (across a layout, a page, and nested components) run the work once and share the result. See its documentation for the details.

# Composing helpers

Any helper can accept `cx: &`[`Cx`], call other helpers, and return a domain-specific result:

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
