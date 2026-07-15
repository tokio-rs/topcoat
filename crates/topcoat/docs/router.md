A [`Router`] handles incoming requests. Build one with [`Router::builder`], register pages, layouts, layers, and API routes, call [`build`](RouterBuilder::build), then pass it to [`start`](crate::start).

For most apps, the recommended way to define routes is the [`module_router!`] macro. It derives the routing table from your module tree instead of defining each URL path by hand.

You can register handlers in two ways: **manually** or with **auto-discovery** (the `discover` feature collects annotated items automatically).

# Paths

Explicit route paths use Topcoat's [`Path`] syntax:

- `/users` for static segments.
- `/users/{id}` for dynamic parameters.
- `/docs/{*path}` for wildcard tails.
- `/(marketing)/pricing` for groups. Groups participate in layout and layer matching but are stripped from the served URL, so this example serves `/pricing`.

The root path is `/`. Non-root paths must start with `/` and may not contain empty segments.

# API routes

An API route is an async function annotated with [`#[route]`](route) and an explicit HTTP method and path:

```rust
use topcoat::{Result, router::route};

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

Route handlers can also read request bodies and return structured responses, the same way for explicit and module-router paths: see [Request and response bodies](#request-and-response-bodies).

# Request and response bodies

A page or route handler can take the request context as `cx: &`[`Cx`](crate::context::Cx) and, alongside it, a single request body parameter. That parameter can be any type that implements [`FromRequest`]. API routes additionally return `Result<T>` where `T:` [`IntoResponse`].

```rust
# #[derive(serde::Deserialize)] struct CreateUser { name: String }
# #[derive(serde::Serialize)] struct User { name: String }
use topcoat::{
    Result,
    context::Cx,
    router::{Json, route},
};

#[route(POST "/api/users")]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<Json<User>> {
    // ...
#     Ok(Json(User { name: input.name }))
}
```

The context and the body parameter are both optional and may appear in either order, but there can be at most one body parameter, because the body is a stream that can only be consumed once. Pages use the same [`FromRequest`] parsing, but return a rendered view rather than an [`IntoResponse`] value.

# Path and query parameters

Two attribute macros declare typed structs for reading values out of a request. You declare a struct, then read it with a free function from any handler that has a `cx`:

- [`#[path_param]`](macro@path_param): one dynamic path segment (like the `{post_id}` in `/posts/{post_id}`), read with [`path_param::<T>(cx)`](fn@path_param).
- [`#[query_params]`](macro@query_params): the request's query string deserialized into a struct, read with [`query_params::<T>(cx)`](fn@query_params).

Both parse lazily and memoize the result for the rest of the request.

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
    view! { /* ... */ }
}
```

See [`#[path_param]`](macro@path_param) and [`#[query_params]`](macro@query_params) for details.

# Pages

A page is an async function annotated with [`#[page]`](page) and an explicit path:

```rust
use topcoat::{Result, router::page, view::view};

#[page("/")]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}

#[page("/about")]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
```

Dynamic and wildcard paths work the same way:

```rust
# use topcoat::{Result, router::page, view::view};
#[page("/users/{id}")]
async fn user_profile() -> Result {
    view! { <h1>"User profile"</h1> }
}

#[page("/docs/{*path}")]
async fn docs_page() -> Result {
    view! { <h1>"Documentation"</h1> }
}
```

# Layouts

A layout wraps pages. It receives a [`Slot`]: a future that resolves to the inner page or layout. Annotate it with [`#[layout]`](layout) and an explicit path:

```rust
use topcoat::{
    Result,
    router::{Slot, layout},
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/about">"About"</a>
                </nav>
                (slot.await?)
            </body>
        </html>
    }
}
```

A layout applies to every page whose path starts with the layout's path. A layout at `/` wraps all pages. A layout at `/settings` wraps `/settings`, `/settings/profile`, `/settings/billing`, and so on.

## Nested layouts

When multiple layouts match a page, they nest from least specific (outermost) to most specific (innermost):

```rust
# use topcoat::{Result, router::{Slot, layout, page}, view::view};
#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! { <html><body>(slot.await?)</body></html> }
}

#[layout("/settings")]
async fn settings_layout(slot: Slot<'_>) -> Result {
    view! {
        <div class="settings-shell">
            <nav>"Settings nav"</nav>
            (slot.await?)
        </div>
    }
}

#[page("/settings/profile")]
async fn profile() -> Result {
    view! { <h1>"Profile"</h1> }
}
```

A request to `/settings/profile` renders: `root_layout` > `settings_layout` > `profile`.

# Layers

A layer wraps matched routes under its path prefix. It receives a mutable request context, the request body, and [`Next`], which represents the remaining layers and the route handler.

```rust
use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Next, Response, layer},
};

#[layer("/")]
async fn timing(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let start = std::time::Instant::now();
    let response = next.run(cx, body).await?;
    println!("handled in {:?}", start.elapsed());
    Ok(response)
}
```

Layer path matching follows the same prefix rule as layouts. A layer at `/` wraps everything, while a layer at `/admin` wraps only routes under `/admin`. Layers at different paths nest from least specific (outermost) to most specific (innermost). If you manually register multiple layers at the same path, the most recently registered layer runs first.

## Tower layers

With the `tower` feature enabled, `TowerLayer` runs middleware from the tower ecosystem (a timeout, a rate limit, CORS, compression) as a layer. It wraps the routes under its path in the middleware a `tower::Layer` builds and registers like any other layer:

```rust,ignore
use std::time::Duration;

use topcoat::router::{Path, Router, TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(
        Path::new("/api"),
        TimeoutLayer::new(Duration::from_secs(5)),
    ))
    .build();
```

See the `TowerLayer` API documentation for the middleware's requirements and error semantics.

# Errors

Every page, layout, layer, and route handler returns a [`Result`](crate::Result). An `Err` becomes the response: the router maps each of its own error types onto an HTTP status code and turns anything else into a 500.

Each status has a constructor:

- [`not_found()`](not_found) responds 404 with [`NotFoundError`].
- [`unauthorized()`](unauthorized) responds 401 with [`UnauthorizedError`].
- [`forbidden()`](forbidden) responds 403 with [`ForbiddenError`].
- [`bad_request(description)`](bad_request) responds 400 with [`BadRequestError`] and a client-safe `description`.
- [`method_not_allowed(methods)`](method_not_allowed) responds 405 with [`MethodNotAllowedError`] and an `Allow` header. The router raises this one itself when a path matches but its method does not.
- [`internal_server_error(error)`](internal_server_error) responds 500 with [`InternalServerError`], wrapping an unexpected error without leaking it to the client.
- [`redirect(uri)`](redirect) and [`redirect_permanent(uri)`](redirect_permanent) respond 307 and 308 with [`RedirectError`].

Each returns a concrete error type that converts into the handler's error, so bubble it up with `?`:

```rust
use topcoat::{Result, context::Cx, router::{not_found, page}, view::view};
# struct Post;
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result {
    let Some(_post) = find_post(cx).await else {
        return Err(not_found().into());
    };
    view! { <h1>"Post"</h1> }
}
```

## From an `Option` or `Result`

Usually the failing value is the condition. [`RouterErrorExt`] adds `ok_or_*` methods to [`Option`] and [`core::result::Result`] that replace `None` (or any `Err`) with a router error, ready for `?`:

```rust
# use topcoat::{Result, context::Cx, router::{RouterErrorExt, page}, view::view};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    view! { <h1>"Dashboard"</h1> }
}
```

The methods cover the same statuses: [`ok_or_not_found`](RouterErrorExt::ok_or_not_found), [`ok_or_unauthorized`](RouterErrorExt::ok_or_unauthorized), [`ok_or_forbidden`](RouterErrorExt::ok_or_forbidden), [`ok_or_bad_request`](RouterErrorExt::ok_or_bad_request), [`ok_or_redirect`](RouterErrorExt::ok_or_redirect), and [`ok_or_redirect_permanent`](RouterErrorExt::ok_or_redirect_permanent). A failed [`#[path_param]`](macro@path_param) parse feeds the same machinery through its `error = ...` option.

# Manual registration

Build a router by chaining `.page()`, `.layout()`, `.layer()`, and `.route()`, then calling [`build`](RouterBuilder::build):

```rust
# use topcoat::{Result, context::CxBuilder, router::{Body, Next, Response, Slot, layer, layout, page, route}, view::view};
# #[layout("/")] async fn root_layout(slot: Slot<'_>) -> Result { view! { (slot.await?) } }
# #[layout("/settings")] async fn settings_layout(slot: Slot<'_>) -> Result { view! { (slot.await?) } }
# #[layer("/")] async fn timing(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> { next.run(cx, body).await }
# #[page("/")] async fn home() -> Result { view! { <h1>"Home"</h1> } }
# #[page("/about")] async fn about() -> Result { view! { <h1>"About"</h1> } }
# #[page("/settings/profile")] async fn profile() -> Result { view! { <h1>"Profile"</h1> } }
# #[route(GET "/api/health")] async fn health() -> Result<&'static str> { Ok("ok") }
use topcoat::router::Router;

pub fn router() -> Router {
    Router::builder()
        .layout(root_layout)
        .layout(settings_layout)
        .layer(timing)
        .page(home)
        .page(about)
        .page(profile)
        .route(health)
        .build()
}
```

Layout-to-page matching is based on path prefixes, not registration order. Layer order is also path-based except when multiple explicitly registered layers share the same path; among those, the last registered layer is outermost and runs first.

# Auto-discovery with `discover()`

With the `discover` feature enabled, every [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](route) is collected at link time. Instead of listing each item by hand, call [`discover`](RouterBuilderDiscoverExt::discover) on the builder:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    Router::builder().discover().build()
}
```

This finds annotated items across your crate and dependencies. Discovered layers must have unique paths because link-time collection order is not stable; if you need to stack several layers on one path, register them explicitly with `.layer(...)`.

# Serving

Use [`start`](crate::start) to run a finalized router:

```rust,no_run
# mod my_app {
#     use topcoat::router::{Router, RouterBuilderDiscoverExt};
#     pub fn router() -> Router { Router::builder().discover().build() }
# }
#[tokio::main]
async fn main() {
    let router = my_app::router();
    topcoat::start(router).await.unwrap();
}
```

[`start`](crate::start) binds to `HOST` and `PORT`, defaulting to `127.0.0.1:3000`. Use [`serve`](crate::serve) when you want to bind the `TcpListener` yourself.

# Example: full manual setup

```rust
use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Json, Next, Response, Router, Slot, layer, layout, page, route},
    view::view,
};

#[derive(serde::Deserialize, serde::Serialize)]
struct NewUser {
    name: String,
}

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/users">"Users"</a>
                </nav>
                (slot.await?)
            </body>
        </html>
    }
}

#[layer("/api")]
async fn api_log(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}

#[page("/")]
async fn home() -> Result {
    view! { <h1>"Welcome"</h1> }
}

#[page("/users")]
async fn users_list() -> Result {
    view! { <h1>"All users"</h1> }
}

#[page("/users/{id}")]
async fn user_profile() -> Result {
    view! { <h1>"User profile"</h1> }
}

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}

// Reads a JSON request body and echoes it back as a JSON response.
#[route(POST "/api/users")]
async fn create_user(Json(user): Json<NewUser>) -> Result<Json<NewUser>> {
    Ok(Json(user))
}

pub fn router() -> Router {
    Router::builder()
        .layout(root_layout)
        .layer(api_log)
        .page(home)
        .page(users_list)
        .page(user_profile)
        .route(health)
        .route(create_user)
        .build()
}
```

# Example: same app with `discover()`

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

// The page, layout, layer, and route definitions are identical. Only the
// router function changes.
pub fn router() -> Router {
    Router::builder().discover().build()
}
```

All [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](route) items from the example above are picked up automatically.
