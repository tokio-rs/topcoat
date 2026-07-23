The `module_router!` macro derives a handler's path from its enclosing Rust module. A handler without a path string uses the module path. When registered, a handler with a path string uses that explicit path.

# Setup

Call `module_router!()` from the root module of the route tree. That module maps to `/`. The macro returns a `RouterBuilder`, so add application context or other router extensions before calling `.build()`.

```rust
// src/app.rs
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!().build()
}
```

`module_router!` uses link-time discovery and requires the `discover` feature. The `topcoat` crate enables it by default.

The macro does not scan the filesystem. Rust must compile each route module through a `mod` declaration:

```text
src/
  app.rs          # contains `mod settings;`
  app/
    settings.rs   # contains `mod profile;`
    settings/
      profile.rs
```

Every module-derived `#[page]`, `#[layout]`, `#[layer]`, and `#[route]` under the module containing `module_router!()` is registered.

# How modules map to routes

Each module below the root contributes one path segment. Static module names are converted to kebab-case.

| Module | Route path |
|---|---|
| `app` | `/` |
| `app::about` | `/about` |
| `app::blog_posts` | `/blog-posts` |
| `app::settings::profile` | `/settings/profile` |

The function name does not affect the path. Two module-derived handlers in the same module receive the same path.

# Pages, layouts, layers, and API routes

A `#[page]` serves `GET` unless the attribute declares other methods, such as `#[page(POST)]`. A `#[layout]` wraps pages in its module and descendant modules.

```rust
# use topcoat::{Result, router::{layout, page}, view::view};
// src/app.rs: both handlers use "/"
#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <html><body>(slot?)</body></html>
    }
}

#[page]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}
```

```rust
# use topcoat::{Result, router::page, view::view};
// src/app/about.rs: GET /about
#[page]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
```

An API route declares one method, a method list, or every method:

```rust
# use topcoat::{Result, router::route};
// src/app/api/health.rs: GET /api/health
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

A layer uses its module path as a prefix:

```rust
// src/app/api.rs: wraps handlers under /api
use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Next, Response, layer},
};

#[layer]
async fn api_log(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```

# Dynamic path parameters

Apply `#[path_param]` to a single-field tuple struct inside the module that should become dynamic. The macro changes that module's segment to a parameter and defines the type used to read it. The snake-cased struct name becomes the parameter name.

```text
src/
  app.rs                  # `mod posts;`
  app/
    posts.rs              # `mod post_id;`, route prefix /posts
    posts/
      post_id.rs          # route /posts/{post_id}
```

```rust
// src/app/posts/post_id.rs
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param},
    view::view,
};

#[path_param(error = bad_request)]
struct PostId(u64);

#[page]
async fn post(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx)?;
    view! { <h1>"Post " (post_id)</h1> }
}
```

This page serves `/posts/{post_id}`. A request for `/posts/42` parses `42` with `u64::from_str`. A failed parse returns `400 Bad Request` because the declaration uses `error = bad_request`.

The parameter name comes from `PostId`, not from the `post_id.rs` filename. The file could be named `id.rs` and would still contribute `{post_id}`.

`path_param::<T>(cx)` returns a request-scoped value:

- `#[path_param] struct Slug(str);` returns the percent-decoded segment as `&str` and cannot fail.
- Any other inner type must implement `FromStr`. Without `error = ...`, the function returns `Result<&T, &T::Err>`.
- `error = bad_request`, `not_found`, `unauthorized`, `forbidden`, `redirect(...)`, or `redirect_permanent(...)` maps a parse failure to that router error.

Parsing occurs once per request. Later calls return the memoized result.

A module contributes one segment, so it can declare one `#[path_param]`. Use nested modules for multiple parameters:

| Module | Route path |
|---|---|
| `app::organizations::organization_id` | `/organizations/{organization_id}` |
| `app::organizations::organization_id::users::user_id` | `/organizations/{organization_id}/users/{user_id}` |

Handlers and layouts in descendant modules can read parameters declared by ancestor modules if the Rust types are visible there.

# Query parameters

Query parameters do not affect module-derived paths. Declare a named-field struct with `#[query_params]`, then read it from any handler that takes `cx: &Cx`.

```rust
# use topcoat::{
#     Result,
#     context::Cx,
#     router::{page, query_params},
#     view::view,
# };
#[query_params(error = bad_request)]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

// In src/app/posts.rs, this serves /posts and accepts
// requests such as /posts?page=2&q=rust.
#[page]
async fn posts(cx: &Cx) -> Result {
    let query = query_params::<PostsQuery>(cx)?;
    view! {
        <p>"page: " (query.page.unwrap_or(1))</p>
        <p>"search: " (query.q.as_deref().unwrap_or(""))</p>
    }
}
```

`#[query_params]` derives `serde::Deserialize`. Use `Option<T>` for optional keys. Parsing occurs once per request and returns a reference to the memoized struct.

# Segment overrides

`segment!(...)` changes the enclosing module's segment. It accepts `kind` and `rename`, each at most once:

| Declaration | Result |
|---|---|
| none in `blog_posts` | `/blog-posts` |
| `segment!(rename = "articles")` | `/articles` |
| `segment!(kind = Group)` | no served URL segment |
| `segment!(kind = Param, rename = "id")` | `/{id}` |
| `segment!(kind = CatchAll, rename = "path")` | `/{*path}` |

`Static` is the default kind for regular modules. `Group` is the default for modules whose names start with `_`. A rename is used as written; Topcoat does not kebab-case it.

`#[path_param]` emits a `Param` segment override, so do not combine it with `segment!` in the same module. A manual `Param` override creates the route capture but does not define a typed accessor.

# Catch-all parameters

A `CatchAll` segment captures one or more remaining URL segments, including their `/` separators. It is the last served segment in a route.

```rust
// src/app/docs/path.rs: /docs/{*path}
use topcoat::{
    Result,
    context::Cx,
    router::{page, raw_path_params, segment},
    view::view,
};

segment!(kind = CatchAll, rename = "path");

#[page]
async fn document(cx: &Cx) -> Result {
    let path = raw_path_params(cx)
        .iter()
        .find_map(|(name, value)| (name == "path").then_some(value))
        .expect("route declares {*path}");

    view! { <p>"Document: " (path)</p> }
}
```

For `/docs/guides/start`, `path` is `"guides/start"`. `/docs` does not match because a catch-all requires a non-empty remainder. Captured values are percent-decoded before the handler reads them.

# Groups

A module whose name starts with `_` contributes a logical group segment but no served URL segment. Layouts and layers still use the group when matching descendants.

```text
app.rs
app/
  _marketing.rs        # layout for this group
  _marketing/
    pricing.rs         # /pricing
    features.rs        # /features
  _docs.rs             # a different layout
  _docs/
    getting_started.rs # /getting-started
```

The two groups can apply different layouts to top-level URLs.

Use an explicit kind when the module name should not select the default:

```rust
// src/app/marketing.rs: hide `marketing` from served URLs.
topcoat::router::segment!(kind = Group);
```

```rust
// src/app/_internal.rs: serve the module at /internal.
topcoat::router::segment!(kind = Static);
```

Group names remain part of Topcoat's logical paths. A layout or layer in `_marketing` applies only to descendants of `_marketing`, even though the group name is absent from request URLs.

# Explicit paths

Adding a path string to `#[page]`, `#[layout]`, `#[layer]`, or `#[route]` disables module path derivation for that item. `segment!` declarations do not alter explicit paths.

`module_router!()` discovers module-derived handlers. Register an explicit-path handler by name:

```rust
# use topcoat::{Result, router::page, view::view};
#[page("/legacy")]
async fn legacy() -> Result {
    view! { <h1>"Legacy"</h1> }
}

pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!()
        .page(legacy)
        .build()
}
```

To discover all explicit-path handlers too, call `RouterBuilderDiscoverExt::discover` on the returned builder:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    topcoat::router::module_router!().discover().build()
}
```

# Conflicts

Handlers in one module share a derived path. They may serve different HTTP methods, but overlapping methods at the same served path are rejected when the router is built. A specific-method route may share a path with a `*` route and takes precedence.

`module_router!` rejects two module-derived layouts or two module-derived layers at the same logical path because link-time discovery does not define their order. To run several layers at one path, register them explicitly with `RouterBuilder::layer`, which gives their order meaning.
