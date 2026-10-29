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

let cx = Cx::new().with(Crumb("Home"));
let posts = cx.with(Crumb("Posts"));

assert_eq!(trail(&posts), "Home / Posts");
assert_eq!(trail(&cx), "Home");
```

# Adding values

[`Cx::with`] creates a child context with a new value. The child can read the new value and every value of its parent. The parent stays unchanged, so only code given the child sees the new value.

Adding a value never removes one. When the context already has a value of the same type, the new value shadows it. Reads return the new value, and the old one stays underneath. This lets code change a default for the code it calls. For example, [`UrlForm`] controls whether URLs are relative to the site root or absolute. The default is relative. Shadow it with `UrlForm::Absolute` for content such as an email body:

```rust
use topcoat::context::{Cx, UrlForm, url_form};

fn email_context(cx: &Cx) -> Cx {
    cx.with(UrlForm::Absolute)
}

let cx = Cx::new();
assert_eq!(url_form(&cx), UrlForm::Relative);
assert_eq!(url_form(&email_context(&cx)), UrlForm::Absolute);
```

A [layer](crate::router#layers) can add values for the rest of the request by passing a child context to [`next.run`](crate::router::Next::run). For data that code can load when it needs it, such as the current user, prefer [helper functions](#functions-not-middlewares).

To add several values at once, pass a tuple to [`Cx::with_many`]. The result is the same as calling `with` for each value, but it creates one child context instead of one per value, so it is more efficient.

# Sharing values across requests

Use [`with`](crate::router::RouterBuilder::with) on the router builder for values that every request should see, such as configuration or a database pool. Requests borrow the same value rather than receiving a copy, so `Clone` is not required. The value must be `Send + Sync + 'static`. To let requests update shared state, store a value that supports changes through a shared reference, such as an atomic or a [`Mutex`](std::sync::Mutex):

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

Without a router, put the shared values in a [`ContextMap`], wrap it in an `Arc`, and attach it to a fresh context with `Cx::new().with_shared(shared)` for each render. See [`Cx::with_shared`]. For tests, start with an empty `Cx::new()` and add the values the test needs with [`Cx::with`].

# Route context

Use route context to declare metadata that layers and layouts can read before the handler runs. Add it with `with(...)` on a [`#[route]`](macro@crate::router::route), [`#[page]`](crate::router::page), or [`#[layout]`](crate::router::layout). The router puts these values in the request context as soon as it selects the route.

For example, a layout can set a default title, and a page can replace it with its own:

```rust
use topcoat::{
    Result,
    context::{Cx, context},
    router::{Router, Slot, layout, page},
    view::{View, view},
};

struct Title(&'static str);

#[layout("/", with(Title("My shop")))]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let Title(title) = context::<Title>(cx);

    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head><title>(title)</title></head>
            <body>(slot)</body>
        </html>
    })
}

#[page("/products", with(Title("Products")))]
async fn products() -> Result<impl View> {
    Ok(view! { <h1>"Products"</h1> })
}

let router = Router::builder().layout(shell).page(products).build();
```

This example uses a custom `Title` to show how route context works. For page titles and other page metadata, use the built-in [`metadata`](crate::metadata) module, which combines values from layouts and pages and renders their HTML tags.

Router builder values are added first, followed by route context, then values added during the request with `cx.with(...)`. Each value shadows earlier values of the same type. For pages, route context follows the layout hierarchy from outermost to innermost, with the page's values last. Repeat `with(...)` to add several values. A request that matches no route has no route context.

Shadowed values remain available through [`iter_context`], which visits the closest value first. This lets pages and layouts contribute metadata that an outer layout collects and renders together. For breadcrumbs, each layout and page could declare a `Crumb` value. The outer layout collects them with `iter_context::<Crumb>(cx)` and reverses the order to render the trail from root to page.

Route context is built once, when the router is built. For a value that depends on the request, such as a crumb showing a customer's name, store a function as a [`BoxCxFn`] and call it while rendering. Calls are not memoized, so load data through [`#[memoize]`](macro@memoize) functions the page uses too.

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

# Functions, not middlewares

Put request logic in small functions that take `cx: &Cx` and call one another. A helper can read request data, load the current user, or check whether that user is allowed to continue. Call it from the page, layout, or component that needs the result. Add [`#[memoize]`](macro@memoize) when several callers should share expensive work. It caches the result for the request and reuses it while the function would still find the same context values:

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

Every render of `user_avatar` checks authentication, and calls for the same user share one database lookup. Each caller asks for the user without knowing whether another caller has already loaded it.

Compare this with authentication middleware, which stores the user on the request so the page can assume the check already ran. If the middleware is missing or runs in the wrong order, the page panics or skips the check. An extractor such as `Auth(user)` puts the requirement in the handler's signature, but the handler then has to pass the user down through components that may never use it. With a helper, the code that needs the user asks for it.
