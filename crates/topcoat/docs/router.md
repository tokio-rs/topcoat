A [`Router`] handles incoming requests. Build one with [`Router::builder`], register pages, layouts, layers, and API routes, call [`build`](RouterBuilder::build), then pass it to [`start`](crate::start).

Handlers register in two ways: **manually**, listing each item on the builder, or with **auto-discovery** (the `discover` feature collects annotated items at link time). For most apps, the recommended way to define routes is the [`module_router!`] macro, which builds on discovery and derives each URL from the module tree instead of a path string.

# Paths

Explicit route paths use Topcoat's [`Path`] syntax:

- `/users` for static segments.
- `/users/{id}` for dynamic parameters.
- `/docs/{*path}` for wildcard tails.
- `/(marketing)/pricing` for groups. Groups participate in layout and layer matching but are stripped from the served URL, so this example serves `/pricing`.

The root path is `/`. Non-root paths must start with `/` and may not contain empty segments.

# Pages

A page is an async function annotated with [`#[page]`](page) and a path, returning a rendered view:

```rust
use topcoat::{Result, router::page, view::view};

#[page("/")]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}

#[page("/users/{id}")]
async fn user_profile() -> Result {
    view! { <h1>"User profile"</h1> }
}
```

A page serves `GET` by default; naming methods before the path (`#[page(POST "/signup")]`) overrides that, with the same method forms as [`#[route]`](route).

See [`#[page]`](page) for the handler signature, module-derived paths, and using pages as components.

# Layouts

A layout wraps pages. It receives the rendered inner page (or nested layout) as a `Result<View>`, to embed in its own view. Annotate it with [`#[layout]`](layout):

```rust
use topcoat::{
    Result,
    router::layout,
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/about">"About"</a>
                </nav>
                (slot?)
            </body>
        </html>
    }
}
```

A layout applies to every page whose path starts with the layout's path: a layout at `/` wraps all pages, while a layout at `/settings` wraps `/settings`, `/settings/profile`, `/settings/billing`, and so on. When multiple layouts match a page, they nest from least specific (outermost) to most specific (innermost). See [`#[layout]`](layout) for the handler signature, nested layouts, and using layouts as components.

# Layers

A layer wraps request handling under its path prefix. It receives a mutable request context, the request body, and [`Next`], which represents the remaining layers and the handler:

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

Layers follow the same prefix rule as layouts and nest from least specific (outermost) to most specific (innermost). See [`#[layer]`](layer) for the exact matching and ordering rules.

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

# API routes

An API route is an async function annotated with [`#[route]`](route) and an explicit HTTP method:

```rust
use topcoat::{Result, router::route};

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

The method can also be a bracketed list (`#[route([GET, POST] "/form")]`) registering the handler for each listed method, or `*` (`#[route(* "/webhook")]`) registering it for every method. A route declaring a specific method takes precedence over a `*` route at the same path.

See [`#[route]`](route) for the handler signature and how return values convert into responses.

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

The context and the body parameter are both optional and may appear in either order, but there can be at most one body parameter, because the body is a stream that can only be consumed once. Pages use the same [`FromRequest`] parsing, but return a rendered view rather than an [`IntoResponse`] value. See [`FromRequest`] and [`IntoResponse`] for the implementing types.

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

# Errors

Every page, layout, layer, and route handler returns a [`Result`](crate::Result). An `Err` becomes the response: the router maps its own error types onto HTTP status codes and turns anything else into a 500.

The [`error`](mod@error) module has a constructor for each response, like [`not_found()`](error::not_found) or [`redirect(uri)`](error::redirect), and the [`RouterErrorExt`](error::RouterErrorExt) methods that turn an `Option` or `Result` into one:

```rust
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::view};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    view! { <h1>"Dashboard"</h1> }
}
```

See the [`error`](mod@error) module docs for how to raise, convert, and catch these errors.

# Status codes and headers

A [`StatusCode`] in a `view!`'s body sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. This pairs with error handling. A layout can catch a page's [`NotFoundError`](error::NotFoundError) and replace it with a branded not-found page:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{
        StatusCode,
        error::{NotFoundError, RouterErrorExt},
        layout, page,
    },
    view::view,
};

# struct Post { title: String }
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result {
    let post = find_post(cx).await.ok_or_not_found()?;
    view! { <h1>(post.title)</h1> }
}

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    let content = match slot {
        Err(error) if error.downcast_ref::<NotFoundError>().is_some() => view! {
            (StatusCode::NOT_FOUND)
            <h1>"Page not found"</h1>
        },
        content => content,
    }?;

    view! {
        <html>
            <body>(content)</body>
        </html>
    }
}
```

See the [`view!`](crate::view::view!) macro docs for the full placement and precedence rules.

# Manual registration

Build a router by chaining `.page()`, `.layout()`, `.layer()`, and `.route()`, then calling [`build`](RouterBuilder::build):

```rust
# use topcoat::{Result, context::CxBuilder, router::{Body, Next, Response, layer, layout, page, route}, view::view};
# #[layout("/")] async fn root_layout(slot: Result) -> Result { view! { (slot?) } }
# #[layout("/settings")] async fn settings_layout(slot: Result) -> Result { view! { (slot?) } }
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

Layout and layer matching is based on path prefixes, not registration order; see [`#[layout]`](layout) and [`#[layer]`](layer) for the ordering rules.

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
    router::{Body, Json, Next, Response, Router, layer, layout, page, route},
    view::view,
};

#[derive(serde::Deserialize, serde::Serialize)]
struct NewUser {
    name: String,
}

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/users">"Users"</a>
                </nav>
                (slot?)
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
