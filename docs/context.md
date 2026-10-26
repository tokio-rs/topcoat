[`Cx`] gives handlers, components, and plain functions access to the values of the current request: the HTTP request itself, values your application shares across requests, and values added while the request is handled.

Add `cx: &Cx` to a function's parameters when it needs context. Topcoat passes it to pages, layouts, routes, layers, and components automatically. Plain functions receive it from their caller.

# How context works

A context holds values identified by their Rust type. Read one with [`context::<T>(cx)`](fn@context):

```rust
use topcoat::context::{Cx, context};

struct Database;

fn db(cx: &Cx) -> &Database {
    context::<Database>(cx)
}
```

Values get into the context in three ways:

1. Values registered on the router with [`with`](crate::router::RouterBuilder::with) are shared by every request. See [sharing values across requests](#sharing-values-across-requests).
2. The router adds values for each request, such as the HTTP request and its path parameters. Topcoat reads them for you through [built-in accessors](#built-in-accessors).
3. Layers, layouts, components, and plain functions add values for the code they call with [`Cx::with`]. See [adding values during a request](#adding-values-during-a-request).

All three are read the same way, so code reading a value does not need to know where it came from.

## Scopes

[`Cx::with`] does not change the context it is called on. It returns a new context that holds the added value and every value its parent can see. Code that receives the new context sees the value. The caller keeps its own context and does not.

When a context can see several values of the same type, the one added last wins. A layer can replace a value registered on the router for the requests it wraps, and a component can replace it again for the views it renders:

```rust
use topcoat::context::{Cx, context};

struct Theme(&'static str);

fn theme(cx: &Cx) -> &'static str {
    context::<Theme>(cx).0
}

let cx = Cx::default().with(Theme("light"));
let inner = cx.with(Theme("dark"));

assert_eq!(theme(&inner), "dark");
assert_eq!(theme(&cx), "light");
```

Values therefore flow down to the code that receives a scope, never back up to its caller or across to its siblings.

## Reading values

[`context`](fn@context) panics when no value of the requested type is visible. Use [`try_context`] when a value is intentionally optional:

```rust
use topcoat::context::{Cx, try_context};

struct Customer;

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_context(cx)
}
```

The requested type must exactly match the registered type. A value registered as `Arc<Database>` is not found when reading `Database`.

Wrap reads in small functions named after what they return, such as `db(cx)`, `config(cx)`, or `current_tenant(cx)`. Callers then depend on the function instead of the type, and a missing value panics in one place.

# Sharing values across requests

## Registering values

Register values that every request needs, such as a database pool or configuration, on the router builder with [`with`](crate::router::RouterBuilder::with):

```rust
use topcoat::router::Router;
#
# struct Database;
# impl Database { fn connect() -> Self { Self } }
# struct HttpClient;
# impl HttpClient { fn new() -> Self { Self } }

fn router() -> Router {
    Router::builder()
        .with(Database::connect())
        .with(HttpClient::new())
        .build()
}
```

[`with_many`](crate::router::RouterBuilder::with_many) registers a tuple of values in one call. Registering a value of a type that is already registered replaces the earlier value. To share two values of the same type, wrap them in distinct types:

```rust
use topcoat::router::Router;
#
# struct Database;
# impl Database { fn connect(_url: &str) -> Self { Self } }

struct PrimaryDb(Database);
struct ReplicaDb(Database);

fn router() -> Router {
    Router::builder()
        .with_many((
            PrimaryDb(Database::connect("postgres://primary")),
            ReplicaDb(Database::connect("postgres://replica")),
        ))
        .build()
}
```

## Shared state

Every request borrows the same registered value, so the value does not need to implement `Clone`. It must be `Send + Sync + 'static`, so it cannot borrow temporary data.

Requests only get shared references. For state that requests change, register a type that can be changed through a shared reference, such as an atomic or a [`Mutex`](std::sync::Mutex):

```rust
use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    context::{Cx, context},
    router::Router,
};

struct PageViews(AtomicU64);

fn router() -> Router {
    Router::builder().with(PageViews(AtomicU64::new(0))).build()
}

fn count_view(cx: &Cx) -> u64 {
    context::<PageViews>(cx).0.fetch_add(1, Ordering::Relaxed) + 1
}
```

## Reading values while building

Code that configures a router, such as an extension trait, can read the values registered so far with [`RouterBuilder::context`](crate::router::RouterBuilder::context) and [`RouterBuilder::try_context`](crate::router::RouterBuilder::try_context). For example, it can register a default only if the application has not registered its own value:

```rust
use topcoat::router::RouterBuilder;

struct Resolver;

fn with_default_resolver(builder: RouterBuilder) -> RouterBuilder {
    if builder.try_context::<Resolver>().is_some() {
        builder
    } else {
        builder.with(Resolver)
    }
}
```

## Without a router

Code that renders views without a router can build the shared values itself. Collect them in a [`ContextMap`] and create a context for each render with [`Cx::new`]:

```rust
use std::sync::Arc;

use topcoat::context::{ContextMap, Cx, context};

struct Site {
    title: &'static str,
}

let mut shared = ContextMap::new();
shared.insert(Site { title: "Topcoat" });
let shared = Arc::new(shared);

let cx = Cx::new(Arc::clone(&shared));
assert_eq!(context::<Site>(&cx).title, "Topcoat");
```

Tests can start from `Cx::default()`, an empty context, and add the values the code under test reads with [`Cx::with`] or [`Cx::with_many`].

# Adding values during a request

## In a layer

A layer usually adds values that the rest of the request needs, such as the current tenant or locale. It passes the new context to [`next.run`](crate::router::Next::run), so every layer, layout, and handler inside it sees the value:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, request::headers, response::Response},
};

struct Tenant(String);

#[layer]
async fn tenant(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let name = headers(cx)
        .get("x-tenant")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("default");
    let cx = cx.with(Tenant(name.to_owned()));
    next.run(&cx, body).await
}
```

Layers apply by path, so a layer under `/admin` can add or replace values for those pages only. See [layers](crate::router#layers) for how they match and nest.

## In plain code

Any function holding a `cx` can derive a child context and pass it on:

```rust
use topcoat::context::{Cx, context};

struct Customer {
    name: String,
}

fn greet(cx: &Cx) -> String {
    let cx = cx.with(Customer {
        name: "Ada".to_owned(),
    });
    format!("Hello, {}", greeting_name(&cx))
}

fn greeting_name(cx: &Cx) -> &str {
    &context::<Customer>(cx).name
}
```

[`Cx::with_many`] adds several values in one step. It behaves like chained `with` calls but builds the child context once.

## Replacing a router value

A value added with `with` replaces a router value of the same type, but only for the code that receives the new context. This lets the router set a default that parts of a request change. For example, URLs render relative to the site root unless the context holds another [`UrlForm`]. Register `UrlForm::Absolute` on the router to make every URL absolute, or add it with `with` only around content that needs complete URLs, such as an email body:

```rust
use topcoat::context::{Cx, UrlForm, url_form};

fn email_context(cx: &Cx) -> Cx {
    cx.with(UrlForm::Absolute)
}

let cx = Cx::default();
assert_eq!(url_form(&cx), UrlForm::Relative);
assert_eq!(url_form(&email_context(&cx)), UrlForm::Absolute);
```

## Values from outside a request

[`Router::handle_with`](crate::router::Router::handle_with) handles a request with extra values in its context, for example when a Tower service or a test calls the router. A [rewrite](crate::router::error#rewrites) starts with a fresh context and can carry values into the next dispatch with [`RewriteError::with`](crate::router::error::RewriteError::with).

# Built-in accessors

Topcoat's own features keep their values in the same context and provide functions to read them, so you rarely call [`context`](fn@context) for them yourself.

## Request data

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

## Path and query parameters

The [`module_param!`](macro@crate::router::module_param) macro and [`#[query_params]`](macro@crate::router::query_params) attribute declare typed values that you read with the [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params) functions. Topcoat parses typed path parameters and query structs lazily and memoizes them for the request.

```rust
// src/app/posts/post_id.rs
use topcoat::{
    Result,
    context::Cx,
    router::{module_param, page, path_param, query_params},
    view::{View, view},
};

module_param!(post_id: uuid::Uuid, error = bad_request);

#[query_params(error = bad_request)]
struct PostQuery {
    preview: Option<bool>,
}

#[page]
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

## Other features

Other features follow the same pattern. For example, [`base_url`] reads the base URL registered with [`RouterBuilder::base_url`](crate::router::RouterBuilder::base_url), and [`url_form`] reads the current [`UrlForm`]. Each feature's documentation lists its accessors.

# Work that outlives the handler

A [`Cx`] is a handle to state shared by everything serving one request. The router drops its own handle once the response is sent, so a streaming response body or a spawned task cannot borrow the `cx` the handler was called with. Clone the `Cx` and move the owned handle into the work instead. It sees the same values as the original.

```rust
// src/app/orders.rs
# async fn record(name: &str) {}
use topcoat::{
    Result,
    context::{Cx, context},
    router::route,
};

struct Customer {
    name: String,
}

#[route(POST)]
async fn place_order(cx: &Cx) -> Result<&'static str> {
    let cx = cx.clone();
    tokio::spawn(async move {
        let customer: &Customer = context(&cx);
        record(&customer.name).await;
    });
    Ok("queued")
}
```

A cloned handle can still read context after the handler returns. It cannot change response headers that have already been sent. Cookie writes after the jar is sealed panic.

# Memoization

[`#[memoize]`](macro@memoize) caches a function's result for one request. Repeated calls with the same arguments share the result when they see the same values for the context types the function read. Use it for expensive work needed in several places. See the macro's documentation for requirements and scoping rules.

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

That keeps pages, layouts, components, and routes focused on rendering or responding while shared request reads stay in ordinary Rust functions. The next section explains why Topcoat prefers these functions over middleware.
