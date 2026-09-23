A [`Router`] maps incoming requests to the code that handles them. Create a builder with [`Router::builder`], register your pages, layouts, layers, and API routes on it, and call [`build`](RouterBuilder::build). Then pass the router to [`start`](crate::start) to serve it.

There are two ways to register handlers: **manually**, by listing each item on the builder, or with **auto-discovery**, where the `discover` feature collects annotated items at link time. For most apps, we recommend the [`module_router!`] macro. It derives each URL from the module tree instead of a path string.

# Paths

Route paths use Topcoat's [`Path`] syntax. A path is made of `/`-separated segments, and each segment is one of four kinds:

- `/users` is a static segment.
- `/{id}` is a dynamic parameter that matches one non-empty segment.
- `/{*path}` is a catch-all parameter that matches one or more remaining segments.
- `/(marketing)` is a group. Groups take part in layout and layer matching, but are not part of the served URL.

The root path is `/`. Every other path starts with `/` and has no empty segments, except that it may end in a `/`. Parameter and group names start with an ASCII letter or `_` and contain only ASCII letters, digits, and underscores.

A trailing slash is part of the path. A page at `/users/` is served at `/users/`, and a page at `/users` is served at `/users`. By default, a request for the other form is redirected to the declared one. Use [`RouterBuilder::trailing_slash`] to change this.

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

A page serves `GET` by default. To serve other methods, name them before the path, like `#[page(POST "/signup")]`. The method forms are the same as for [`#[route]`](macro@route).

See [`#[page]`](page) for the handler signature, module-derived paths, and using pages as components.

# Layouts

A layout wraps pages. It receives the inner page (or the next nested layout) as a [`Slot`] and places it in its own view. Annotate it with [`#[layout]`](layout):

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

A layer runs code around the handling of every request under its path prefix. It receives the request context, the request body, and [`Next`], which stands for the remaining layers and the handler. To make request-scoped values available to the code below it, a layer derives a child context with [`Cx::with`](crate::context::Cx::with) and passes it to `next.run`:

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

The method can also be a bracketed list, like `#[route([GET, POST] "/form")]`, which registers the handler for each listed method, or `*`, like `#[route(* "/webhook")]`, which registers it for every method. A route with a specific method takes precedence over a `*` route at the same path.

See [`#[route]`](macro@route) for the handler signature and how return values convert into responses.

# Request and response bodies

A page or route handler can take the request context as `cx: &`[`Cx`](crate::context::Cx) and, next to it, one request body parameter, such as a [`Json`](content::Json) body. An API route also returns a value that becomes the response.

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

The context and the body parameter are both optional and can appear in either order. There can be at most one body parameter, because the body is a stream that can only be read once. Pages read bodies the same way, but return a rendered view instead of a response value. See the [`content`](mod@content) module docs for the available extractors and response types, including multipart uploads, WebSockets, and server-sent events.

# Path and query parameters

Path and query values are read from [`Cx`](crate::context::Cx), not passed as handler arguments. This keeps handler signatures short, and lets helper functions and layouts read the same parameters.

## Path parameters

Call [`path_param!`](macro@path_param) with the parameter name from the URL. The macro generates a type named after the parameter in upper camel case, so `path_param!(post_id: u64)` declares `PostId` for `{post_id}`:

- After `path_param!(slug)`, `path_param::<Slug>(cx)` returns the percent-decoded segment as `&str`.
- A type after `:` is parsed with [`FromStr`](std::str::FromStr). The default return type is `Result<&T, &<T as FromStr>::Err>`.
- `error = bad_request`, `not_found`, `unauthorized`, `forbidden`, `redirect(...)`, or `redirect_permanent(...)` maps a parse failure to that router error.

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

The value is parsed once per request, and later reads reuse the result.

Prefix the name with `*` to capture the remaining path as decoded segments. After `path_param!(*doc_path)`, `path_param::<DocPath>(cx)` returns [`CatchAllSegments`]. After `path_param!(*ids: u32)`, `path_param::<Ids>(cx)` returns `Result<&[u32], _>`.

With [`module_router!`], a declaration inside a non-root route module also changes that module's segment to the parameter. See [`module_router!`] for module structure, nested parameters, and catch-all parameters.

## Query parameters

Apply [`#[query_params]`](macro@query_params) to a struct with named fields. The macro derives `serde::Deserialize`, and [`query_params::<T>(cx)`](fn@query_params) deserializes the request query string into that struct. Use `Option<T>` for keys that may be absent.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::{View, view},
};

#[query_params(error = bad_request)]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PostsQuery>(cx)?;
    Ok(view! {
        <p>"page: " (query.page.unwrap_or(1))</p>
        <p>"search: " (query.q.as_deref().unwrap_or(""))</p>
    })
}
```

The query string is also parsed once per request, and each read returns a reference to the stored value. A query struct does not depend on the matched route, so any function with `cx: &Cx` can read it. See [`#[query_params]`](macro@query_params) for error handling and redirecting after invalid input.

# Errors

Every page, layout, layer, and route handler returns a [`Result`](crate::Result). An `Err` becomes the response. The router maps its own error types to HTTP status codes and turns any other error into a 500 response.

The [`error`](mod@error) module has a constructor for each response, like [`not_found()`](error::not_found) or [`redirect(uri)`](error::redirect), and the [`RouterErrorExt`](error::RouterErrorExt) methods that turn an `Option` or `Result` into one:

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

A [`StatusCode`] placed in a `view!` body sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. This is useful together with error handling. For example, a layout can wrap its slot in an [`error_boundary`](crate::view::error_boundary) to catch a page's [`NotFoundError`](error::NotFoundError) and render a custom not-found page instead:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{
        Slot, StatusCode,
        error::{NotFoundError, RouterErrorExt},
        layout, page,
    },
    view::{View, error_boundary, view},
};

# struct Post { title: String }
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post = find_post(cx).await.ok_or_not_found()?;
    Ok(view! { <h1>(post.title)</h1> })
}

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <html>
            <body>
                error_boundary(
                    fallback: |error| {
                        if error.downcast_ref::<NotFoundError>().is_none() {
                            // Any other error type is rethrown.
                            return Err(error);
                        }

                        Ok(view! {
                            (StatusCode::NOT_FOUND)
                            <h1>"Page not found"</h1>
                        })
                    },
                    (slot)
                )
            </body>
        </html>
    })
}
```

See the [`view!`](crate::view::view!) macro docs for the full placement and precedence rules, and the [`error`](mod@error) module docs for catching errors.

# Cross-origin requests

The router applies an [`OriginPolicy`] to every request before any layer or handler runs. By default, it rejects state-changing cross-origin browser requests and cross-origin WebSocket handshakes with `403 Forbidden`. Register a different policy with [`origin_policy`](RouterBuilder::origin_policy) to trust other origins, to exempt individual routes, or to turn the check off. See [`OriginPolicy`] for the exact rules.

# Manual registration

To register handlers by hand, chain `.page()`, `.layout()`, `.layer()`, and `.route()` calls on the builder, then call [`build`](RouterBuilder::build):

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

Layouts and layers are matched by path prefix, not by registration order. See [`#[layout]`](layout) and [`#[layer]`](layer) for the ordering rules.

# Auto-discovery with `discover()`

With the `discover` feature (enabled by default), every [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](macro@route) is collected at link time. Instead of listing each item by hand, call [`discover`](RouterBuilderDiscoverExt::discover) on the builder:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    Router::builder().discover().build()
}
```

This finds annotated items in your crate and its dependencies. Discovered layouts and discovered layers must each have unique paths, because the order of link-time collection is not stable. To stack several layers on one path, register them by hand with `.layer(...)`.

Other features also collect items at link time, and `discover()` registers those too, such as fonts declared with `font!` and the runtime's procedures and shards. Values that are not annotated items are always registered by hand, like the asset bundle (`.assets(...)`), app context (`.app_context(...)`), and the runtime's own routes (`.runtime()`, see the [runtime guide](../runtime/index.html#setup)).

[`module_router!`] registers only the handlers it derives from the module tree. It returns a `RouterBuilder`, so call `discover()` on it or register the remaining items by hand, as shown above.

# Static files

For application assets such as images and stylesheets, we recommend the [asset system](../asset/index.html). It generates content-hashed URLs that browsers can cache for a long time, and its Rust asset handles avoid typos in URL strings. Serving a directory is useful when files need fixed URLs or are created at runtime.

Enable the `fs` feature to serve files from a directory. Import [`RouterBuilderDirectoryExt`] and call [`public_dir`](RouterBuilderDirectoryExt::public_dir) to serve them at the site root:

```rust
# #[cfg(feature = "fs")]
# {
use topcoat::router::{Router, RouterBuilderDirectoryExt};

let router = Router::builder().public_dir("./public").build();
# }
```

This serves `./public/logo.svg` at `/logo.svg` and `./public/css/site.css` at `/css/site.css`. A relative directory is resolved against the current working directory of the process. The root URL `/` stays free for a home page.

Use [`serve_dir`](RouterBuilderDirectoryExt::serve_dir) to serve a directory at a different route path. The path must end in a catch-all parameter, which selects the file within the directory:

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

Use [`start`](crate::start) to serve a built router:

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

[`start`](crate::start) listens on the address given by the `HOST` and `PORT` environment variables, and on `127.0.0.1:3000` when they are not set. Use [`serve`](crate::serve) to bind the listener yourself. It accepts any [`Listener`]: a `TcpListener` to serve HTTP directly, or on Unix a `UnixListener` to serve behind a reverse proxy (like nginx or Caddy) that forwards requests to a socket path:

```rust,no_run
# #[cfg(unix)]
# async fn serve(router: topcoat::router::Router) -> std::io::Result<()> {
let path = "/run/my-app.sock";
let _ = std::fs::remove_file(path);
let listener = tokio::net::UnixListener::bind(path)?;
topcoat::serve(listener, router).await
# }
```

The socket file from a previous run is not removed automatically, so remove any old file before binding, as above.

Both functions shut the server down gracefully when the process receives Ctrl+C or, on Unix, `SIGTERM`. Use [`serve_until`](crate::serve_until) to shut down on a signal of your own.

The HTTP server uses Tokio and Hyper and is behind the `serve` Cargo feature, which is enabled by default. Routing, views, and request handling work without it: [`Router::handle`] turns a [`Request`](request::Request) into a [`Response`](response::Response) directly, with no listener involved. On a platform that receives HTTP requests for you, such as a serverless or WebAssembly runtime, build `topcoat` without default features, leave `serve` off, and call [`Router::handle`] from the platform's request handler.

# Tower services

With the `tower` feature enabled, the [`tower`](mod@tower) module connects the router to the [tower](https://docs.rs/tower) ecosystem. [`TowerRoute`](tower::TowerRoute) mounts a tower service (like an axum router) as a route, and [`TowerLayer`](tower::TowerLayer) runs tower middleware as a layer. See the [`tower`](mod@tower) module docs for details.

# Example: full manual setup

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Next, Router, Slot, content::Json, layer, layout, page, response::Response, route,
    },
    view::{View, view},
};

#[derive(serde::Deserialize, serde::Serialize)]
struct NewUser {
    name: String,
}

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/users">"Users"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}

#[layer("/api")]
async fn api_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Welcome"</h1> })
}

#[page("/users")]
async fn users_list() -> Result<impl View> {
    Ok(view! { <h1>"All users"</h1> })
}

#[page("/users/{id}")]
async fn user_profile() -> Result<impl View> {
    Ok(view! { <h1>"User profile"</h1> })
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

`discover()` picks up all [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](macro@route) items from the example above.
