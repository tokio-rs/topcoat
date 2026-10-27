[`Cx`] holds the values your code needs while handling a request. These include HTTP request data, shared application values such as a database connection, and values added for part of the request.

Add a `cx: &Cx` parameter when a function needs these values. Topcoat supplies it to pages, layouts, routes, layers, and components. When you call a plain function, pass `cx` yourself.

# Reading values

A context holds values by their Rust type. Use [`context::<T>(cx)`](fn@context) to borrow the value of type `T`. It panics if the context has none. When a value may be missing, use [`try_context`] to get an `Option`:

```rust
use topcoat::context::{Cx, context, try_context};

struct Database;
struct Customer;

fn db(cx: &Cx) -> &Database {
    context(cx)
}

fn current_customer(cx: &Cx) -> Option<&Customer> {
    try_context(cx)
}
```

Use the exact type you added. If you added an `Arc<Database>`, ask for `Arc<Database>`, not `Database`.

Small helpers such as `db(cx)` and `current_customer(cx)` give these reads clear names. They also keep the choice of stored type and the handling of missing values in one place. Topcoat's own values come with helpers like these, such as [`headers(cx)`](crate::router::request::headers) for request data. See [`router::request`](crate::router::request) and the documentation of each feature.

A context can hold several values of one type, because a new value [shadows](#adding-values) older ones instead of removing them. `context` returns the closest one. [`iter_context`] visits every value of the type, starting with the closest one and walking back through the values it shadows. This suits types that several scopes add to, such as breadcrumbs:

```rust
use topcoat::context::{Cx, iter_context};

struct Crumb(&'static str);

fn trail(cx: &Cx) -> String {
    let mut crumbs: Vec<_> = iter_context::<Crumb>(cx).map(|crumb| crumb.0).collect();
    crumbs.reverse();
    crumbs.join(" / ")
}

let cx = Cx::default().with(Crumb("Home"));
let posts = cx.with(Crumb("Posts"));

assert_eq!(trail(&posts), "Home / Posts");
assert_eq!(trail(&cx), "Home");
```

# Adding values

Adding a value never removes one. When a context already has a value of the same type, the new value shadows it. Reads return the newest value, and the older one stays underneath.

[`Cx::with`] creates a child context with the new value on top. The child can read the new value and every value of its parent. The parent stays unchanged. Pass the child to any code that should see the new value. For example, a layer can shadow a default, and a component can shadow it again for its own content:

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

Only code given the child context sees its new value. The caller and other children keep reading their own values.

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

To add several values at once, pass a tuple to [`Cx::with_many`]. The result is the same as calling `with` for each value, but it creates one child context instead of one per value.

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

# Functions, not middlewares

Put request checks in functions that take `cx: &Cx`. A helper can read request data, load the current user, or check whether that user is allowed to continue.

Call the helper from the code that needs the check or its result. Add `#[memoize]` when several callers should share expensive work within a request.

## What not to do

### Middleware

With authentication middleware, the check and the code using the user often live in different places. The middleware stores a user on the request. The page then reads that value and assumes the check has already happened:

```rust
# struct Request { extensions: Extensions }
# struct Extensions;
# impl Extensions { fn insert<T>(&mut self, _v: T) {} fn get<T>(&self) -> Option<&T> { None } }
# struct Html;
# struct User;
# async fn authenticate(_: &Request) -> User { User }
# fn render_account(_: &User) -> Html { Html }
async fn auth_middleware(request: &mut Request) {
    let user = authenticate(request).await;
    request.extensions.insert(user);
}

async fn account_page(request: &Request) -> Html {
    let user = request
        .extensions
        .get::<User>()
        .expect("auth middleware must have run");

    render_account(user)
}
```

For this page to work, the right middleware must run first. Missing it or setting the wrong order can cause a panic or leave a route without its authentication check.

### Extractors

An extractor puts the authentication requirement in the handler's parameters:

```rust
# struct Auth(User);
# struct User;
# struct Html;
# fn render_account(_: User) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render_account(user)
}
```

The handler now states that it needs an authenticated user. Components below it still need the user passed to them:

```rust
# struct Auth(User);
# #[derive(Clone)] struct User { avatar_url: u8 }
# struct Html;
# fn render(_: Html, _: Html) -> Html { Html }
# async fn account_settings(_: User) -> Html { Html }
# fn render_image(_: u8) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render(
        account_sidebar(user.clone()).await,
        account_settings(user.clone()).await,
    )
}

async fn account_sidebar(user: User) -> Html {
    user_avatar(user).await
}

async fn user_avatar(user: User) -> Html {
    render_image(user.avatar_url)
}
```

Passing values this way is useful for data chosen by the caller. For data shared across a request, it can mean passing a value through components that never use it themselves.

## What to do in Topcoat

Write small helpers that take `&Cx` and call one another. Any page, layout, or component can use them:

```rust
use topcoat::{
    context::{context, memoize, Cx},
    router::{error::{RouterErrorExt, UnauthorizedError}, request::headers},
    Result,
};

# #[derive(Clone)] struct Db;
# struct User;
# struct FetchBuilder;
# impl User { fn fetch_by_id(_: &str) -> FetchBuilder { FetchBuilder } }
# impl FetchBuilder { async fn exec(self, _: Db) -> Option<User> { None } }
#
/// Clones the database handle stored in the context.
fn db(cx: &Cx) -> Db {
    context::<Db>(cx).clone()
}

/// Loads a user by ID and caches the result for this request.
#[memoize(as_ref)]
async fn fetch_user(cx: &Cx, user_id: &str) -> Option<User> {
    User::fetch_by_id(user_id).exec(db(cx)).await
}

/// Finds the session cookie in the request.
fn session_cookie(cx: &Cx) -> Option<&str> {
    let headers = headers(cx);
    // ... extract session cookie from HTTP headers
    None
}

/// Looks up the user named by the session, if present.
async fn fetch_current_user(cx: &Cx) -> Option<&User> {
    let user_id = session_cookie(cx)?;
    fetch_user(cx, user_id).await
}

/// Returns the user, or an unauthorized error if there is none.
async fn require_auth(cx: &Cx) -> Result<&User, UnauthorizedError> {
    fetch_current_user(cx).await.ok_or_unauthorized()
}
```

`#[memoize(as_ref)]` stores the `Option<User>` for the request and gives callers an `Option<&User>`. Helpers can borrow the user without cloning it or passing ownership from one component to the next.

A component that needs an authenticated user calls `require_auth(cx)` itself:

```rust
use topcoat::{
    context::Cx,
    view::{View, component, view},
    Result,
};

# use topcoat::router::error::UnauthorizedError;
# struct User { avatar_url: &'static str, name: &'static str }
# async fn require_auth(_: &Cx) -> Result<&User, UnauthorizedError> { Err(topcoat::router::error::unauthorized()) }
#
/// Checks authentication and renders the user's avatar.
#[component]
async fn user_avatar(cx: &Cx) -> Result<impl View> {
    let user = require_auth(cx).await?;

    Ok(view! {
        <img
            src=(user.avatar_url)
            alt=(format!("{}'s avatar", user.name))
        >
    })
}
```

Every render of `user_avatar` checks authentication. If the request has no valid session, the component returns an unauthorized error. Other components and handlers that need protection must also call the helper.

Calls with the same user ID and context dependencies share the database lookup. Each caller can ask for the user without knowing whether another caller has already loaded it.
