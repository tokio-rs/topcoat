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

# App, request, and scoped context

This module exposes typed context accessors:

- [`app_context::<T>(cx)`](app_context) reads a required value registered on the router with `.app_context(value)`.
- [`try_app_context::<T>(cx)`](try_app_context) reads an optional value registered on the router.
- [`request_context::<T>(cx)`](request_context) reads the nearest required request or scoped value.
- [`try_request_context::<T>(cx)`](try_request_context) reads the nearest optional request or scoped value.

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

Use [`Cx::with`] to create a child context that adds or shadows one value. The child borrows its parent and owns the new binding:

```rust
use topcoat::context::{Cx, request_context};

#[derive(Debug, PartialEq)]
struct HeadingLevel(u8);

fn section(cx: &Cx) {
    let section_cx = cx.with(HeadingLevel(2));

    assert_eq!(request_context::<HeadingLevel>(&section_cx), &HeadingLevel(2));
    assert_eq!(try_request_context::<HeadingLevel>(cx), None);
}
# use topcoat::context::try_request_context;
```

[`Cx::with_values`] adds each element of a tuple as a separate binding. It accepts tuples of two through twelve values and panics if one call contains duplicate types. `cx.with((a, b))` is different: it stores `(a, b)` as one tuple binding.

Request lookup starts at the nearest scope, continues through its parents, and checks the request root last. Other types remain inherited when one type is shadowed. Dropping a [`CxScope`] removes that scope from reach without changing its parent.

# Mutating request context

Layers receive `&mut Cx`, so they can register root values with [`Cx::insert`] and mutate existing ones with [`Cx::get_mut`]. A scope provides shared access only, and Rust prevents root mutation while a scope or borrowed result may still be used. Root mutation is available again after [`Next::run`](crate::router::Next::run) completes.

# Memoization

[`#[memoize]`](macro@memoize) keys results by function arguments and the request bindings read during computation. Scoped bindings can select different cached results, while changing one root type invalidates only results that read it. See the macro documentation for dependency and concurrency details.

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
