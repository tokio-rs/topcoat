[`Cx`] is Topcoat's request context. It gives access to the current request, to values shared by the whole app, and to values scoped to the current request.

Pages, layouts, layers, components, and routes receive it when their signature has a `cx: &Cx` parameter. Leave the parameter out when a function does not need it. Any ordinary function can take a `cx: &Cx` too, which is how you write reusable helpers that read request data (see [Functions, not middlewares](#functions-not-middlewares)).

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

The most common ones are:

- [`parts(cx)`](crate::router::request::parts) returns the current request's `http::request::Parts`.
- [`method(cx)`](crate::router::request::method) returns the HTTP method.
- [`uri(cx)`](crate::router::request::uri) returns the request URI.
- [`version(cx)`](crate::router::request::version) returns the HTTP version.
- [`headers(cx)`](crate::router::request::headers) returns the request headers.
- [`content_type(cx)`](crate::router::request::content_type) returns the request `Content-Type`.
- [`extensions(cx)`](crate::router::request::extensions) returns request extensions.
- [`remote_addr(cx)`](crate::router::request::remote_addr) returns the address of the direct connection, if it has one.
- [`client_ip(cx)`](crate::router::request::client_ip) returns the client's IP address, read through any trusted reverse proxies.

Use [`parts(cx)`](crate::router::request::parts) when you need several fields at once:

```rust
use topcoat::{context::Cx, router::request::parts};

fn cache_key(cx: &Cx) -> String {
    let parts = parts(cx);
    format!("{}:{}", parts.method, parts.uri)
}
```

Use [`extensions(cx)`](crate::router::request::extensions) to read typed values that a lower-level layer or service attached to the HTTP request:

```rust
use topcoat::{context::Cx, router::request::extensions};

struct RequestId(String);

fn request_id(cx: &Cx) -> Option<&str> {
    extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
}
```

A handler reached through a [rewrite](crate::router::error#rewrites) sees the rewritten request in these helpers. Each HTTP field helper has an `original_` counterpart, like [`original_parts(cx)`](crate::router::request::original_parts) or [`original_uri(cx)`](crate::router::request::original_uri), that returns the request as the client sent it. Layers can also change the current request without a rewrite. For example, [`StripPrefixLayer`](crate::router::StripPrefixLayer) changes `uri(cx)` while `original_uri(cx)` keeps the incoming URI. The client's IP address is resolved when the request arrives and stays the same across rewrites.

# Path and query helpers

The [`path_param!`](macro@crate::router::path_param) macro and the [`#[query_params]`](macro@crate::router::query_params) attribute declare typed values. Read them with the [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params) functions. Topcoat parses each value the first time it is read and reuses the result for the rest of the request.

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

Any function with a `cx: &Cx` can read these values, not only the page itself. See the [router guide](crate::router#path-and-query-parameters) for the details.

# App and request context helpers

The app context holds values registered once on the router, like a database pool. The request context holds values scoped to the current request, like the signed-in user. Both are keyed by Rust type:

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

The required helpers panic when no value of the requested type is registered. Wrap them in small application-specific functions like `db(cx)`, `config(cx)`, or `current_tenant(cx)`, so the type and the expectation live in one place.

Use the `try_` helpers when a value may be missing:

```rust
use topcoat::context::{Cx, try_request_context};
#
# struct Customer;

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_request_context(cx)
}
```

# Registering request context

Request context is registered by scoping: [`Cx::with`] returns a child `Cx` whose request context also holds the given value, and [`Cx::with_many`] registers a tuple of values in one step. The child inherits every other value and shares the rest of the request state, such as the app context and the memoize cache, with its parent.

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

Registering a type that is already present shadows it in the child scope. Lookups through the child see the new value, while lookups through the parent still see the original. Router layers use this to make values like the cookie jar available to everything below them: they derive a child context and pass it to the rest of the chain.

# Work that outlives the handler

A [`Cx`] is a handle to the state of one request. A streaming response body or a spawned task can outlive the handler, so it cannot borrow the `cx` the handler was called with. Clone the `Cx` and move the clone into the work instead. The clone reads the same app and request context.

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

A clone can still read the context after the response was sent, but it can no longer change what the client receives. Cookie changes and other writes aimed at the response are dropped when they happen after the handler returned.

# Base URL and URL form

Some rendered content leaves the site, like links in an email or a sitemap, and needs absolute URLs. Register the public address of your app on the router with `.base_url(...)` and read it back with [`base_url(cx)`](base_url()) or [`try_base_url(cx)`](try_base_url). [`BaseUrl::join`] resolves a path against it.

[`url_form(cx)`](url_form()) returns the [`UrlForm`] in effect: URLs render relative unless an enclosing scope registers [`UrlForm::Absolute`] with [`Cx::with`].

# Memoization

[`#[memoize]`](macro@memoize) caches the result of a function that takes `cx` for the rest of the request, keyed by its arguments. Use it on helpers that do expensive work, so that repeated calls from a layout, a page, and nested components run the work once and share the result. When a memoized function reads request context, it keeps a separate result for each set of values it saw, so a cached value is never used outside the scope it was computed in. See [`#[memoize]`](macro@memoize) for the details.

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

This keeps pages, layouts, components, and routes focused on rendering or responding, while shared request logic stays in ordinary Rust functions.
