A [`Router`] matches incoming requests to handlers. Define a page, register it with [`Router::builder`], and call [`build`](RouterBuilder::build):

```rust
use topcoat::{Result, router::{Router, page}, view::{View, view}};

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Home"</h1> })
}

let router = Router::builder().page(home).build();
```

Pass the router to [`start`](crate::start) to serve requests. As your application grows, [`module_router!`] can derive route paths from your Rust modules.

# Paths

Explicit route paths use Topcoat's [`Path`] syntax. A path is made of `/`-separated segments, and each segment is one of four kinds.

- `/users` is a static segment.
- `/{id}` is a dynamic parameter that matches one non-empty segment.
- `/{*path}` is a catch-all parameter that matches one or more remaining segments.
- `/(marketing)` is a group. Groups take part in layout and layer matching but are stripped from the served URL.

The root path is `/`. Every other path starts with `/` and has no empty segments, except that it may end in a `/`. Parameter and group names start with an ASCII letter or `_` and contain only ASCII letters, digits, and underscores.

A trailing slash is part of the path. A page at `/users/` is served at `/users/`, and a page at `/users` is served at `/users`. A request for the other form is redirected to the declared one by default. Use [`RouterBuilder::trailing_slash`] to configure this behavior.

# Pages

A page is an async function annotated with [`#[page]`](page) and a path, returning a rendered view:

```rust
use topcoat::{Result, router::page, view::{View, view}};

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Home"</h1> })
}

#[page("/users/{id}")]
async fn user_profile() -> Result<impl View> {
    Ok(view! { <h1>"User profile"</h1> })
}
```

A page serves `GET` by default; naming methods before the path (e.g., `#[page(POST "/signup")]`) overrides that, with the same method forms as [`#[route]`](macro@route).

See [`#[page]`](page) for the handler signature, module-derived paths, and using pages as components.

# Layouts

A layout wraps pages. It receives the inner page (or nested layout) as a [`Slot`], to embed in its own view. Annotate it with [`#[layout]`](layout):

```rust
use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/about">"About"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}
```

A layout applies to every page whose path starts with the layout's path: a layout at `/` wraps all pages, while a layout at `/settings` wraps `/settings`, `/settings/profile`, `/settings/billing`, and so on. When multiple layouts match a page, they nest from least specific (outermost) to most specific (innermost). See [`#[layout]`](layout) for the handler signature, nested layouts, and using layouts as components.

# Layers

A layer runs around request handling under its path prefix. Call [`Next::run`] to run the remaining layers and handler. For example, this layer measures how long they take:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, response::Response},
};

#[layer("/")]
async fn timing(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let start = std::time::Instant::now();
    let response = next.run(cx, body).await?;
    println!("handled in {:?}", start.elapsed());
    Ok(response)
}
```

Layers follow the same prefix rule as layouts and nest from least specific (outermost) to most specific (innermost). A layer whose [`path`](Layer::path) is `None` wraps every request, including one that matches no route: the 404 or 405 error comes back through it as the `Err` returned by `next.run`. See [`#[layer]`](layer) for the exact matching and ordering rules.

# API routes

An API route is an async function annotated with [`#[route]`](macro@route) and an explicit HTTP method:

```rust
use topcoat::{Result, router::route};

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

The method can also be a bracketed list (`#[route([GET, POST] "/form")]`) registering the handler for each listed method, or `*` (`#[route(* "/webhook")]`) registering it for every method. A route declaring a specific method takes precedence over a `*` route at the same path.

See [`#[route]`](macro@route) for the handler signature and how return values convert into responses.

# Request and response bodies

A page or route handler can take the request context as `cx: &`[`Cx`](crate::context::Cx) and, alongside it, a single request body parameter, such as a [`Json`](content::Json) body. An API route additionally returns a value that becomes the response.

```rust
# #[derive(serde::Deserialize)] struct CreateUser { name: String }
# #[derive(serde::Serialize)] struct User { name: String }
use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, route},
};

#[route(POST "/api/users")]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<Json<User>> {
    // ...
#     Ok(Json(User { name: input.name }))
}
```

The context and the body parameter are both optional and may appear in either order, but there can be at most one body parameter, because the body is a stream that can only be consumed once. Pages parse bodies the same way, but return a rendered view rather than a response value. See [`content`](mod@content) for request parsing and response types.

# Path and query parameters

Read URL parameters from the request context. Declare a typed path parameter with [`path_param!`](macro@path_param), then read it with [`path_param`](fn@path_param):

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param},
    view::{View, view},
};

path_param!(post_id: u64, error = bad_request);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { <h1>"Post " (post_id)</h1> })
}
```

Here, `/posts/42` gives the handler `42`. A value that cannot be parsed as `u64` produces `400 Bad Request`. See [`path_param!`](macro@path_param) for string parameters, catch-all paths, and error handling.

For query strings, declare a struct with [`#[query_params]`](macro@query_params) and read it with [`query_params::<T>(cx)`](fn@query_params). Use optional fields for keys that may be absent. The macro reference shows a complete example.

# Errors

Handlers return a [`Result`](crate::Result). A router error selects an HTTP response, such as `404 Not Found`. An unhandled error of another type becomes `500 Internal Server Error`.

Use [`RouterErrorExt`](error::RouterErrorExt) to turn an absent or failed value into a router error:

```rust
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::{View, view}};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

See the [`error`](mod@error) module docs for how to raise, convert, and catch these errors.

# Status codes and headers

A [`StatusCode`] in a `view!` body sets the response status. A [`HeaderMap`] or a `(HeaderName, HeaderValue)` pair adds response headers. See [`view!`](crate::view::view!) for placement and precedence.

A layout can use [`error_boundary`](crate::view::error_boundary) to show a replacement view when a page fails. Set the replacement's status explicitly, or it will be served as a successful response. See [`error`](mod@error) for a complete example.

# Cross-origin requests

The router applies an [`OriginPolicy`] to every request before any layer or handler runs: by default, state-changing cross-origin browser requests and cross-origin WebSocket handshakes are rejected with `403 Forbidden`. Register a policy with [`origin_policy`](RouterBuilder::origin_policy) to trust cross-origin peers, to exempt individual routes, or to opt out; see [`OriginPolicy`] for the exact rules.

# Manual registration

Build a router by chaining `.page()`, `.layout()`, `.layer()`, and `.route()`, then calling [`build`](RouterBuilder::build):

```rust
# use topcoat::{Result, context::Cx, router::{Body, Next, Slot, layer, layout, page, response::Response, route}, view::{View, view}};
# #[layout("/")] async fn root_layout(slot: Slot<'_>) -> Result<impl View> { Ok(view! { (slot) }) }
# #[layout("/settings")] async fn settings_layout(slot: Slot<'_>) -> Result<impl View> { Ok(view! { (slot) }) }
# #[layer("/")] async fn timing(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> { next.run(cx, body).await }
# #[page("/")] async fn home() -> Result<impl View> { Ok(view! { <h1>"Home"</h1> }) }
# #[page("/about")] async fn about() -> Result<impl View> { Ok(view! { <h1>"About"</h1> }) }
# #[page("/settings/profile")] async fn profile() -> Result<impl View> { Ok(view! { <h1>"Profile"</h1> }) }
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

With the `discover` feature enabled, every [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](macro@route) is collected at link time. Instead of listing each item by hand, call [`discover`](RouterBuilderDiscoverExt::discover) on the builder:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    Router::builder().discover().build()
}
```

This finds annotated items across your crate and dependencies. Discovered layers must have unique paths because link-time collection order is not stable; if you need to stack several layers on one path, register them explicitly with `.layer(...)`.

`discover()` also runs registration hooks provided by enabled features. Follow each feature's setup instructions for any values or configuration it needs on the builder.

[`module_router!`] registers module-derived handlers only. It returns a `RouterBuilder`, so call `discover()` on it, or register the remaining items by hand, exactly as above.

# Static files

For application assets such as images and stylesheets, we recommend the [asset system](../asset/index.html). It generates content-hashed URLs for long-lived caching, and Rust asset handles help avoid typos in URL strings. Serving a directory is useful when files need fixed URLs or are created at runtime.

Enable the `fs` feature to serve files from a directory. Import [`RouterBuilderDirectoryExt`] and call [`public_dir`](RouterBuilderDirectoryExt::public_dir) to serve them at the site root:

```rust
# #[cfg(feature = "fs")]
# {
use topcoat::router::{Router, RouterBuilderDirectoryExt};

let router = Router::builder().public_dir("./public").build();
# }
```

This serves `./public/logo.svg` at `/logo.svg` and `./public/css/site.css` at `/css/site.css`. The directory is relative to the process's current working directory. The root URL `/` is left available for a home page.

Use [`serve_dir`](RouterBuilderDirectoryExt::serve_dir) to choose a different route path. Its final catch-all parameter selects the file within the directory:

```rust
# #[cfg(feature = "fs")]
# {
use topcoat::router::{Router, RouterBuilderDirectoryExt};

let router = Router::builder()
    .serve_dir("/downloads/{*file}", "./files")
    .build();
# }
```

This serves `./files/report.pdf` at `/downloads/report.pdf`. The catch-all does not match `/downloads` or `/downloads/` itself.

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

[`start`](crate::start) binds to `HOST` and `PORT`, defaulting to `127.0.0.1:3000`. Use [`serve`](crate::serve) when you want to bind the listener yourself. It accepts any [`Listener`]: a `TcpListener` to serve HTTP directly, or on Unix a `UnixListener` to serve behind a reverse proxy (like nginx or Caddy) that forwards requests to a socket path:

```rust,no_run
# #[cfg(unix)]
# async fn serve(router: topcoat::router::Router) -> std::io::Result<()> {
let path = "/run/my-app.sock";
let _ = std::fs::remove_file(path);
let listener = tokio::net::UnixListener::bind(path)?;
topcoat::serve(listener, router).await
# }
```

The socket file of a previous run is not removed automatically, so remove any stale file before binding, as above.

The `serve` Cargo feature enables the HTTP server and is enabled by default. Without it, [`Router::handle`] turns a [`Request`](request::Request) into a [`Response`](response::Response) directly, with no listener involved. On a platform that receives HTTP requests for you, such as a serverless or WebAssembly runtime, build `topcoat` without default features, leave `serve` off, and call [`Router::handle`] from the platform's request handler.

# Tower services

With the `tower` feature enabled, the [`tower`](mod@tower) module bridges the tower ecosystem: [`TowerRoute`](tower::TowerRoute) mounts a tower service (like an axum router) as a route, and [`TowerLayer`](tower::TowerLayer) runs tower middleware as a layer. See the [`tower`](mod@tower) module docs for details.
