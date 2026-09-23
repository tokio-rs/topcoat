[`Cx`] gives you access to the current request and the values shared with it.

Add `cx: &Cx` to the function signature when needed; leave it out when the function does not need request context. Topcoat passes it automatically when the parameter is present.

# Router request helpers

Use [`router::request`](crate::router::request) to read HTTP request data from `cx`.

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

A layer or [rewrite](crate::router::error#rewrites) can change the current request. Use [`original_parts(cx)`](crate::router::request::original_parts) to read the request parts as they arrived, or [`original_uri(cx)`](crate::router::request::original_uri) when you only need the incoming URI.

# Path and query helpers

Declare typed URL parameters with [`path_param!`](macro@crate::router::path_param) and [`#[query_params]`](macro@crate::router::query_params). Read them from `cx` with the matching functions.

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

Use [`app_context`] to read a value shared across requests. Register it on the router with `.app_context(value)`. Use [`request_context`] for a value in the current request scope.

```rust
use topcoat::context::{Cx, app_context};
#
# struct Database;

fn db(cx: &Cx) -> &Database {
    app_context::<Database>(cx)
}
```

Values are looked up by their Rust type. These helpers panic if the type is missing. A helper like `db(cx)` gives the lookup a name that fits your app.

Use [`try_app_context`] or [`try_request_context`] when a value is optional. They return `None` if the type is missing:

```rust
use topcoat::context::{Cx, try_request_context};
#
# struct Customer;

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_request_context(cx)
}
```

# Registering request context

Use [`Cx::with`] to create a child context with an additional value. The child inherits the other values and shares the request state with its parent. Use [`Cx::with_many`] to add several values at once.

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

If the type is already present, lookups through the child see the new value. The parent keeps its original value. Pass the child context to any code that should use the new value.

# Work that outlives the handler

Clone `cx` when you need to move it into work that outlives the handler. The clone shares the same app and request context and keeps those values available.

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

Use [`#[memoize]`](macro@memoize) when repeated calls to a helper should share its result within a request. The cache accounts for the function's arguments and the request context values it reads. See the macro's documentation for examples and caching rules.

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

Call these helpers wherever you have a `&Cx`.
