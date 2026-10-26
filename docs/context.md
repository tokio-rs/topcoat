[`Cx`] holds the values your code needs while handling a request. These include HTTP request data, shared application values such as a database connection, and values added for part of the request.

Add a `cx: &Cx` parameter when a function needs these values. Topcoat supplies it to pages, layouts, routes, layers, and components. When you call a plain function, pass `cx` yourself.

# How context works

Each value in a context is stored under its Rust type. Use [`context::<T>(cx)`](fn@context) to borrow the value stored under `T`:

```rust
use topcoat::context::{Cx, context};

struct Database;

fn db(cx: &Cx) -> &Database {
    context::<Database>(cx)
}
```

There are three ways to add values:

1. Call [`with`](crate::router::RouterBuilder::with) on the router builder to [share a value across requests](#sharing-values-across-requests).
2. The router supplies each request's HTTP data and path parameters. Read these through Topcoat's [built-in accessors](#built-in-accessors).
3. Call [`Cx::with`] while handling a request to [add a value for the code you call next](#adding-values-during-a-request).

Read all of these values with the same functions. The code reading a value does not need to know how it was added.

## Scopes

[`Cx::with`] creates a child context. The child can read the new value and the values in its parent. The parent stays unchanged. Pass the child to any code that should see the new value.

Adding another value of the same type replaces that value in the child. For example, a layer can override a router default, and a component can override it again for its own content:

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

Only code given the child context sees its changes. The caller and other children keep their own values.

## Reading values

[`context`](fn@context) panics if the context has no value of that type. When a value may be missing, use [`try_context`] to get an `Option`:

```rust
use topcoat::context::{Cx, try_context};

struct Customer;

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_context(cx)
}
```

Use the exact type you registered. If you added an `Arc<Database>`, ask for `Arc<Database>`, not `Database`.

Small helper functions such as `db(cx)`, `config(cx)`, and `current_tenant(cx)` give these reads clear names. They also keep the choice of stored type and the handling of missing values in one place.

# Sharing values across requests

## Registering values

Use [`with`](crate::router::RouterBuilder::with) on the router builder for values that requests should share, such as configuration or a database pool:

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

Use [`with_many`](crate::router::RouterBuilder::with_many) to add a tuple of values at once. Adding a type again replaces its earlier value. To keep two values of the same underlying type, give each one a different wrapper type:

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

Requests borrow the same value rather than receiving a copy, so `Clone` is not required. The value must be `Send + Sync + 'static`. It cannot borrow data that will be dropped while the router still needs it.

To let requests update shared state, store a value that supports changes through a shared reference. An atomic or a [`Mutex`](std::sync::Mutex) can do this:

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

While building a router, read values already added with [`RouterBuilder::context`](crate::router::RouterBuilder::context) or [`RouterBuilder::try_context`](crate::router::RouterBuilder::try_context). This is useful in setup helpers that should supply a default only when the application has not supplied a value:

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

You can also create a context without a router. Put the shared values in a [`ContextMap`], wrap it in an `Arc`, and call [`Cx::new`] for each render:

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

For tests, start with an empty `Cx::default()`. Add the values the test needs with [`Cx::with`] or [`Cx::with_many`].

# Adding values during a request

## In a layer

A layer can set values such as the current tenant or locale before calling the rest of the request. Pass its child context to [`next.run`](crate::router::Next::run) so the inner layers, layouts, and handler can read those values:

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

A layer's path determines which requests it wraps. For example, a layer under `/admin` can override values just for that part of the app. See [layers](crate::router#layers) for the matching and ordering rules.

## In plain code

Plain functions can create and pass child contexts too:

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

To add several values together, use [`Cx::with_many`]. It has the same result as calling `with` for each value, but creates the child context in one step.

## Replacing a router value

Router values can serve as defaults that a child context overrides. For example, [`UrlForm`] controls whether URLs are relative to the site root or absolute. The default is relative. Add `UrlForm::Absolute` to the router to use absolute URLs throughout the app, or to a child context for content such as an email body:

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

When calling the router yourself, use [`Router::handle_with`](crate::router::Router::handle_with) to supply extra context values. A [rewrite](crate::router::error#rewrites) creates a fresh context. Use [`RewriteError::with`](crate::router::error::RewriteError::with) for values that should reach the rewritten request.

# Built-in accessors

Topcoat stores its own values in the context too. It provides helper functions to read them, so you usually do not need to call [`context`](fn@context) directly for these values.

## Request data

Read HTTP request data with the functions in [`router::request`](crate::router::request):

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

For a single part of the request, use a helper such as [`headers(cx)`](crate::router::request::headers). The [`router::request`](crate::router::request) module lists the others.

Use [`parts(cx)`](crate::router::request::parts) to read several request fields together:

```rust
use topcoat::{context::Cx, router::request::parts};

fn cache_key(cx: &Cx) -> String {
    let parts = parts(cx);
    format!("{}:{}", parts.method, parts.uri)
}
```

Use [`extensions(cx)`](crate::router::request::extensions) to read values attached to the HTTP request by a service or request layer:

```rust
use topcoat::{context::Cx, router::request::extensions};

struct RequestId(String);

fn request_id(cx: &Cx) -> Option<&str> {
    extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
}
```

The HTTP helpers read the current request, including changes made by a [rewrite](crate::router::error#rewrites) or a layer. Use the `original_` helpers, such as [`original_parts(cx)`](crate::router::request::original_parts) and [`original_uri(cx)`](crate::router::request::original_uri), to read what the client sent. For example, [`StripPrefixLayer`](crate::router::StripPrefixLayer) changes `uri(cx)` but leaves `original_uri(cx)` unchanged. Topcoat determines the client IP address when the request arrives and keeps it across rewrites.

## Path and query parameters

Declare path parameters with [`module_param!`](macro@crate::router::module_param) and query fields with [`#[query_params]`](macro@crate::router::query_params). Read them with [`path_param::<T>(cx)`](fn@crate::router::path_param) and [`query_params::<T>(cx)`](fn@crate::router::query_params). Topcoat parses them when first needed and caches the results for the request.

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

These helpers work in any function that receives `&Cx`.

## Other features

Other features provide similar helpers. For example, [`base_url`] reads the URL set with [`RouterBuilder::base_url`](crate::router::RouterBuilder::base_url), and [`url_form`] reads the context's [`UrlForm`]. Look in each feature's documentation for its helpers.

# Work that outlives the handler

A stream or spawned task may keep running after its handler returns. It cannot keep borrowing the handler's `cx`. Clone the [`Cx`] and move that clone into the stream or task. The clone keeps the request state alive and reads the same values:

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

The clone remains usable after the handler returns, but it cannot change headers already sent to the client. Writing cookies after the jar is sealed panics.

# Memoization

Use [`#[memoize]`](macro@memoize) when several parts of a request need the result of the same expensive work. It caches a function's result for that request. Calls can reuse a result when their arguments match and the context values the function read have not been replaced. The macro's documentation explains the requirements and how child contexts affect reuse.

# Composing helpers

Give your own helpers a `cx: &`[`Cx`] parameter to read request data or call other helpers:

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

Put common request logic in these functions and call them from pages, layouts, components, or routes. The next section shows how to use this approach for authentication and other request checks.
