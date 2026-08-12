[`Cx`] is Topcoat's request context. Pages, layouts, components, and routes can take it as an optional parameter when they need request-scoped information.

Add `cx: &Cx` to the function signature when needed; leave it out when the function does not need request context. Topcoat passes it automatically when the parameter is present.

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

The ones you reach for most, all listed in [`topcoat::router::request`](crate::router::request):

- [`parts(cx)`](crate::router::request::parts) returns the current request's `http::request::Parts`.
- [`method(cx)`](crate::router::request::method) returns the HTTP method.
- [`uri(cx)`](crate::router::request::uri) returns the request URI.
- [`version(cx)`](crate::router::request::version) returns the HTTP version.
- [`headers(cx)`](crate::router::request::headers) returns the request headers.
- [`content_type(cx)`](crate::router::request::content_type) returns the request `Content-Type`.
- [`extensions(cx)`](crate::router::request::extensions) returns request extensions.

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

# Path and query helpers

The [`path_param!`](macro@crate::router::path_param) macro and [`#[query_params]`](macro@crate::router::query_params) attribute declare typed values that you read with the [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params) functions. Topcoat parses typed path parameters and query structs lazily and memoizes them for the request.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, query_params},
    view::view,
};

path_param!(post_id: uuid::Uuid, error = bad_request);

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

Registering a type that is already present shadows it for the child scope: lookups through the child see the new value, while lookups through the parent still see the original. This is how router layers make values like the cookie jar available to everything below them; they derive a child context and pass it to the rest of the chain.

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

A cloned handle keeps reading the context after the response was sent, but it can no longer change what the client receives. Cookie changes and other response-directed writes made from work that outlives the handler are dropped.

# Memoization

[`#[memoize]`](macro@memoize) caches a `cx`-taking function's result for the duration of a request, keyed by its arguments. Wrap the request helpers above with it so that repeated calls (across a layout, a page, and nested components) run the work once and share the result. See its documentation for the details.

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
