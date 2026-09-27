A [`Router`] matches requests to handlers. You can derive paths from Rust modules or write them directly on handlers. [Module routing](macro@module_router) is the recommended default. Both approaches build the same router, which you pass to [`start`](crate::start) to serve requests.

# Module routing

Call [`module_router!`] in the root of your route tree, then [`build`](RouterBuilder::build). Handlers omit path strings and take their paths from the enclosing modules:

```rust,standalone_crate
use topcoat::router::{Router, module_router};

// src/app.rs: this module maps to /.
pub fn router() -> Router {
    module_router!().build()
}
```

Declare child modules with `mod`, just as in any Rust application. A `#[page]` in `app::about` serves `/about`, and one in `app::settings::profile` serves `/settings/profile`. The function name does not affect the URL. See the [module routing guide](macro@module_router) for setup, parameters, groups, and segment overrides.

# Explicit paths

To choose paths directly, put an absolute path in each handler's attribute and register the handlers on [`Router::builder`]. Their Rust module locations do not affect the URLs:

```rust
use topcoat::{
    Result,
    router::{Router, page, route},
    view::{View, view},
};

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Home"</h1> })
}

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}

pub fn router() -> Router {
    Router::builder().page(home).route(health).build()
}
```

Layouts and layers work the same way: `#[layout("/settings")]` and `#[layer("/api")]` declare path prefixes, and `.layout(...)` and `.layer(...)` register them. See [manual registration](#manual-registration) for all four handler kinds together.

With the `discover` feature enabled, `Router::builder().discover().build()` collects explicit-path handlers automatically. Import [`RouterBuilderDiscoverExt`] to use it. This changes registration only; you still write paths in the attributes. See [auto-discovery](#auto-discovery-with-discover) for details.

You can also combine the approaches. `module_router!()` collects module-derived handlers; chain `.page(home)` or `.route(health)` to add an explicit-path handler, or `.discover()` to collect them all.

# Linking to pages and routes

Use [`href!`](macro@href) for URLs targeting Topcoat handlers, including navigation links, form actions, and redirects. Pass the handler's Rust name so the URL follows its registered path when you rename a module or change an explicit path. Pass each path parameter as a `path_param!` value:

```rust
use topcoat::{router::href, view::view};
# use topcoat::{Result, router::{page, route}, view::View};
# #[page("/posts")] async fn posts() -> Result<impl View> { Ok(topcoat::view::view! { "Posts" }) }
# #[route(POST "/posts")] async fn publish() -> Result<()> { Ok(()) }
# mod post_id {
#     use super::*;
#     topcoat::router::path_param!(pub post_id: u64);
#     #[page("/posts/{post_id}")] pub async fn post() -> Result<impl View> { Ok(topcoat::view::view! { "Post" }) }
# }
# #[page] async fn example() -> Result<impl View> {
Ok(view! {
    <a href=(href!(posts))>"All posts"</a>
    <a href=(href!(post_id::post, post_id::PostId(1)))>"First post"</a>
    <form method="post" action=(href!(publish))>
        <button>"Publish"</button>
    </form>
})
# }
# fn main() {}
```

A view renders the returned [`Href`] directly. Call [`resolve`](Href::resolve) when a redirect helper needs a string:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{error::{SeeOther, see_other}, href, route},
};
# #[topcoat::router::page("/posts")]
# async fn posts() -> Result<impl topcoat::view::View> { Ok(topcoat::view::view! { "Posts" }) }

#[route(POST)]
async fn publish(cx: &Cx) -> Result<SeeOther> {
    // Save the post, then redirect to the list.
    Ok(see_other(href!(posts).resolve(cx)))
}
```

Use ordinary URLs for external destinations and standalone fragment links such as `href="#details"`. The [`href!` guide](macro@href) covers queries, fragments, and URL forms. See [layouts](#layouts) below for active navigation with [`Href::is_current`] and [`class!`](macro@crate::view::class).

# Path syntax

Explicit route paths use Topcoat's [`Path`] syntax. A path is made of `/`-separated segments, and each segment is one of four kinds.

- `/users` is a static segment.
- `/{id}` is a dynamic parameter that matches one non-empty segment.
- `/{*path}` is a catch-all parameter that matches one or more remaining segments.
- `/(marketing)` is a group. Groups take part in layout and layer matching but are stripped from the served URL.

The root path is `/`. Every other path starts with `/` and has no empty segments, except that it may end in a `/`. Parameter and group names start with an ASCII letter or `_` and contain only ASCII letters, digits, and underscores.

A trailing slash is part of the path. A page at `/users/` is served at `/users/`, and a page at `/users` is served at `/users`. A request for the other form is redirected to the declared one by default. Use [`RouterBuilder::trailing_slash`] to configure this behavior.

# Pages

A page is an async function annotated with [`#[page]`](page), returning a rendered view:

```rust
// src/app.rs
use topcoat::{Result, router::page, view::{View, view}};

#[page]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Home"</h1> })
}

mod users {
    mod id {
        use topcoat::{Result, router::{page, path_param}, view::{View, view}};

        path_param!(id);

        #[page]
        async fn user_profile() -> Result<impl View> {
            Ok(view! { <h1>"User profile"</h1> })
        }
    }
}
```

A page serves `GET` by default. Naming methods, such as `#[page(POST)]`, overrides that, with the same method forms as [`#[route]`](macro@route).

See [`#[page]`](page) for the handler signature, module-derived paths, and using pages as components.

# Layouts

A layout wraps pages. It receives the inner page (or nested layout) as a [`Slot`], to embed in its own view. Annotate it with [`#[layout]`](layout):

```rust
// src/app.rs
use topcoat::{
    Result,
    context::Cx,
    router::{Slot, href, layout},
    view::{View, class, view},
};
# #[topcoat::router::page] async fn home() -> Result<impl View> { Ok(topcoat::view::view! { "Home" }) }
# mod about {
#     use super::*;
#     #[topcoat::router::page] pub async fn page() -> Result<impl View> { Ok(topcoat::view::view! { "About" }) }
# }

#[layout]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let about_link = href!(about::page);
    let current = about_link.is_current(cx);

    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href=(href!(home))>"Home"</a>
                    <a
                        href=(about_link)
                        aria-current=(current.then_some("page"))
                        class=(class!("nav-link", "active" if current))
                    >
                        "About"
                    </a>
                </nav>
                (slot)
            </body>
        </html>
    })
}
# fn main() {}
```

A layout matches a page by path segments. A layout at `/` wraps all pages. One at `/settings` wraps `/settings` and pages below it, such as `/settings/profile`. Matching layouts nest with the least specific path outside the most specific one. See [`#[layout]`](layout) for details.

The navigation uses [`Href::is_current`] to identify the active link and [`class!`](macro@crate::view::class) to combine its base and conditional classes.

# Layers

A layer wraps request handling under its path prefix. It receives the request context, the request body, and [`Next`], which represents the remaining layers and the handler. A layer that registers request-scoped values derives a child context with [`Cx::with`](crate::context::Cx::with) and passes that to `next.run`:

```rust
// src/app.rs
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, response::Response},
};

#[layer]
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
// src/app/api/health.rs
use topcoat::{Result, router::route};

#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

The method can also be a bracketed list (`#[route([GET, POST])]`) registering the handler for each listed method, or `*` (`#[route(*)]`) registering it for every method. A route declaring a specific method takes precedence over a `*` route at the same path.

See [`#[route]`](macro@route) for the handler signature and how return values convert into responses.

# Request and response bodies

A page or route handler can take the request context as `cx: &`[`Cx`](crate::context::Cx) and, alongside it, a single request body parameter, such as a [`Json`](content::Json) body. An API route additionally returns a value that becomes the response.

```rust
// src/app/api/users.rs
# #[derive(serde::Deserialize)] struct CreateUser { name: String }
# #[derive(serde::Serialize)] struct User { name: String }
use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, route},
};

#[route(POST)]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<Json<User>> {
    // ...
#     Ok(Json(User { name: input.name }))
}
```

Both parameters are optional and may appear in either order. A handler can take only one body parameter because the body can be consumed only once. Pages parse bodies the same way and return a view. See [`content`](mod@content) for request extractors and response types.

# Path and query parameters

Read path and query values from [`Cx`](crate::context::Cx). Any helper with access to the context can read them.

## Path parameters

Call [`path_param!`](macro@path_param) with the parameter name from the URL. The macro generates a Pascal-cased type, so `path_param!(post_id: u64)` declares `PostId` for `{post_id}`:

- After `path_param!(slug)`, `path_param::<Slug>(cx)` returns the percent-decoded segment as `&str`.
- A type after `:` is parsed with [`FromStr`](std::str::FromStr). The default return type is `Result<&T, &<T as FromStr>::Err>`.
- An `error = ...` option maps parse failures to a router error. See [`path_param!`](macro@path_param) for the supported forms.

```rust
// src/app/posts/post_id.rs
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param},
    view::{View, view},
};

path_param!(post_id: u64, error = bad_request);

#[page]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { <h1>"Post " (post_id)</h1> })
}
```

Parsing occurs once per request and the result is memoized.

Prefix the name with `*` to capture the remaining path as decoded segments. After `path_param!(*doc_path)`, `path_param::<DocPath>(cx)` returns [`CatchAllSegments`]. After `path_param!(*ids: u32)`, `path_param::<Ids>(cx)` returns `Result<&[u32], _>`.

With [`module_router!`], a declaration inside a non-root route module also changes that module's segment to the parameter. See [`module_router!`] for module structure, nested parameters, and catch-all parameters.

## Query parameters

Apply [`#[query_params]`](macro@query_params) to a struct with named fields. The macro derives `serde::Deserialize`, and [`query_params::<T>(cx)`](fn@query_params) deserializes the request query string into that struct. Use `Option<T>` for keys that may be absent.

```rust
// src/app/posts.rs
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

#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PostsQuery>(cx)?;
    Ok(view! {
        <p>"page: " (query.page.unwrap_or(1))</p>
        <p>"search: " (query.q.as_deref().unwrap_or(""))</p>
    })
}
```

Query parsing also occurs once per request and returns a reference to the memoized value. A query struct is independent of the matched route, so any handler with `cx: &Cx` can read it. See [`#[query_params]`](macro@query_params) for error handling and redirecting after invalid input.

# Errors

Handlers return a [`Result`](crate::Result). The router converts an unhandled error into an HTTP response. Router errors select a status code, while other errors produce `500 Internal Server Error`.

The [`error`](mod@error) module has a constructor for each response, like [`not_found()`](error::not_found) or [`redirect(uri)`](error::redirect), and the [`RouterErrorExt`](error::RouterErrorExt) methods that turn an `Option` or `Result` into one:

```rust
// src/app/dashboard.rs
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::{View, view}};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

See the [`error`](mod@error) module docs for how to raise, convert, and catch these errors.

# Status codes and headers

A [`StatusCode`] in a `view!`'s body sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. This pairs with error handling. Wrapping the slot in an [`error_boundary`](crate::view::error_boundary) lets a layout catch a page's [`NotFoundError`](error::NotFoundError) and replace it with a branded not-found page:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{
        Slot, StatusCode,
        error::{NotFoundError, RouterErrorExt},
        layout, page, path_param,
    },
    view::{View, error_boundary, view},
};

# struct Post { title: String }
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
mod posts {
    mod id {
        use super::super::*;

        path_param!(id);

        #[page]
        async fn post(cx: &Cx) -> Result<impl View> {
            let post = find_post(cx).await.ok_or_not_found()?;
            Ok(view! { <h1>(post.title)</h1> })
        }
    }
}

#[layout]
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
# fn main() {}
```

See the [`view!`](crate::view::view!) macro docs for the full placement and precedence rules, and the [`error`](mod@error) module docs for catching errors.

# Cross-origin requests

The router applies an [`OriginPolicy`] to every request before any layer or handler runs: by default, state-changing cross-origin browser requests and cross-origin WebSocket handshakes are rejected with `403 Forbidden`. Register a policy with [`origin_policy`](RouterBuilder::origin_policy) to trust cross-origin peers, to exempt individual routes, or to opt out; see [`OriginPolicy`] for the exact rules.

# Manual registration

For routes with explicit paths, register each handler with `.page()`, `.layout()`, `.layer()`, or `.route()`, then call [`build`](RouterBuilder::build):

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

With the `discover` feature enabled, explicit-path [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](macro@route) handlers can be collected at link time. Instead of listing each item by hand, call [`discover`](RouterBuilderDiscoverExt::discover) on the builder:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    Router::builder().discover().build()
}
```

This finds annotated items across your crate and dependencies. Discovered layers must have unique paths because link-time collection order is not stable; if you need to stack several layers on one path, register them explicitly with `.layer(...)`.

Discovery also registers items collected by enabled integrations. It does not replace their setup steps. Follow each integration's guide to register any configuration or services it requires.

[`module_router!`] registers module-derived handlers only. It returns a `RouterBuilder`, so call `discover()` on it, or register the remaining items by hand, exactly as above.

# Static files

For application assets such as images and stylesheets, we recommend the [asset system](../asset/index.html). It generates content-hashed URLs for long-lived caching, and Rust asset handles help avoid typos in URL strings. Serving a directory is useful when files need fixed URLs or are created at runtime.

Enable the `fs` feature to serve files from a directory. Import [`RouterBuilderDirectoryExt`] and call [`public_dir`](RouterBuilderDirectoryExt::public_dir) to serve them at the site root:

```rust,standalone_crate
# #[cfg(feature = "fs")]
# {
use topcoat::router::{RouterBuilderDirectoryExt, module_router};

let router = module_router!().public_dir("./public").build();
# }
```

This serves `./public/logo.svg` at `/logo.svg` and `./public/css/site.css` at `/css/site.css`. The directory is relative to the process's current working directory. The root URL `/` is left available for a home page.

Use [`serve_dir`](RouterBuilderDirectoryExt::serve_dir) to choose a different route path. Its final catch-all parameter selects the file within the directory:

```rust,standalone_crate
# #[cfg(feature = "fs")]
# {
use topcoat::router::{RouterBuilderDirectoryExt, module_router};

let router = module_router!()
    .serve_dir("/downloads/{*file}", "./files")
    .build();
# }
```

This serves `./files/report.pdf` at `/downloads/report.pdf`. The catch-all does not match `/downloads` or `/downloads/` itself.

# Serving

Use [`start`](crate::start) to run a finalized router:

```rust,no_run
# mod my_app {
#     use topcoat::router::{Router, RouterBuilderDiscoverExt, module_router};
#     pub fn router() -> Router { module_router!().discover().build() }
# }
#[tokio::main]
async fn main() {
    let router = my_app::router();
    topcoat::start(router).await.unwrap();
}
```

[`start`](crate::start) reads `HOST` and `PORT`, with `127.0.0.1:3000` as the default. Use [`serve`](crate::serve) to supply your own [`Listener`]. On Unix, a `UnixListener` can accept requests forwarded by a reverse proxy:

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

The HTTP server requires the `serve` feature, which is enabled by default. When another platform receives requests for you, call [`Router::handle`] to turn a [`Request`](request::Request) into a [`Response`](response::Response). This works without `serve`.

# Tower services

With the `tower` feature enabled, the [`tower`](mod@tower) module bridges the tower ecosystem: [`TowerRoute`](tower::TowerRoute) mounts a tower service (like an axum router) as a route, and [`TowerLayer`](tower::TowerLayer) runs tower middleware as a layer. See the [`tower`](mod@tower) module docs for details.

# Example: full manual setup

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Next, Router, Slot, content::Json, href, layer, layout, page, response::Response, route,
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
                    <a href=(href!(home))>"Home"</a>
                    <a href=(href!(users_list))>"Users"</a>
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

All [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](macro@route) items from the example above are picked up automatically.
