The `module_router!` macro builds routes from your Rust module tree. Each module becomes a path segment, so a page in `app::settings::profile` is served at `/settings/profile`. A handler without a path string is served at its module's path. A handler whose path string starts with `./` is served below its module's path. A handler with an absolute path string, such as `"/about"`, is not affected by the module tree.

# Setup

Call `module_router!()` in the root module of your route tree. That module maps to `/`. The macro returns a `RouterBuilder`, so you can register anything else your application needs on it before calling `.build()`.

```rust
// src/app.rs
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!().build()
}
```

`module_router!` finds handlers at link time and needs the `discover` feature. The `topcoat` crate enables this feature by default.

The macro does not scan the filesystem. It only sees modules that Rust compiles, so every route module needs a `mod` declaration:

```text
src/
  app.rs          # contains `mod settings;`
  app/
    settings.rs   # contains `mod profile;`
    settings/
      profile.rs
```

`module_router!()` registers every `#[page]`, `#[layout]`, `#[layer]`, and `#[route]` without an absolute path. All of these must be declared in the module that calls `module_router!()` or in one of its descendants. A module-derived handler declared anywhere else in the program makes the macro panic.

# Registering everything else

`module_router!()` only registers module-derived handlers. Everything else goes on the builder it returns, the same way as on a builder from `Router::builder()`. This includes handlers with an absolute path, fonts, procedures, shards, the asset bundle, and app context values.

With the `discover` feature, `RouterBuilderDiscoverExt::discover` registers every item Topcoat collects at link time. This covers handlers with an absolute path and the items of other features, such as fonts. Registration adds to what is already there, so it combines with `module_router!()`:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    topcoat::router::module_router!().discover().build()
}
```

Values have to be passed in by hand. The most common one is the asset bundle. A view that renders an `Asset`, such as a Tailwind stylesheet or a self-hosted font, needs the bundle on the router:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

pub fn router() -> Router {
    topcoat::router::module_router!()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build()
}
```

A missing registration is not a compile error. It shows up on the first request that renders the item, as a panic about a type that is not registered in the app context. The type named in the panic tells you which registration is missing.

# How modules map to routes

Each module below the root adds one path segment. Static module names are converted to kebab-case.

| Module | Route path |
|---|---|
| `app` | `/` |
| `app::about` | `/about` |
| `app::blog_posts` | `/blog-posts` |
| `app::settings::profile` | `/settings/profile` |

The function name does not affect the path. Two module-derived handlers in the same module get the same path.

# Pages, layouts, layers, and API routes

A `#[page]` serves `GET` unless the attribute names other methods, as in `#[page(POST)]`. A `#[layout]` wraps the pages in its module and in all descendant modules.

```rust
# use topcoat::{Result, router::{Slot, layout, page}, view::{View, view}};
// src/app.rs: both handlers use "/"
#[layout]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <html><body>(slot)</body></html>
    })
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"Home"</h1> })
}
```

```rust
# use topcoat::{Result, router::page, view::{View, view}};
// src/app/about.rs: GET /about
#[page]
async fn about() -> Result<impl View> {
    Ok(view! { <h1>"About"</h1> })
}
```

An API route names one method, a list of methods, or every method with `*`:

```rust
# use topcoat::{Result, router::route};
// src/app/api/health.rs: GET /api/health
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

A layer wraps every handler under its module path:

```rust
// src/app/api.rs: wraps handlers under /api
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, response::Response},
};

#[layer]
async fn api_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```

# Relative paths

A path string that starts with `./` is added to the end of the module path. Use it to serve a handler below its module without creating a new module for it.

```rust
# use topcoat::{Result, router::page, view::{View, view}};
// src/app/settings.rs: GET /settings
#[page]
async fn settings() -> Result<impl View> {
    Ok(view! { <h1>"Settings"</h1> })
}

// src/app/settings.rs: POST /settings/export
#[page(POST "./export")]
async fn export() -> Result<impl View> {
    Ok(view! { <p>"Export started"</p> })
}
```

Relative paths can also add a trailing slash. A bare `./` serves the module path with a trailing slash, and a relative path that ends in a slash keeps it. The root module has no trailing slash form, so `./` cannot be used there.

```rust
# use topcoat::{Result, router::page, view::{View, view}};
// src/app/settings.rs: GET /settings/
#[page("./")]
async fn settings() -> Result<impl View> {
    Ok(view! { <h1>"Settings"</h1> })
}

// src/app/settings.rs: GET /settings/export/
#[page("./export/")]
async fn export() -> Result<impl View> {
    Ok(view! { <h1>"Export"</h1> })
}
```

`#[layout]`, `#[layer]`, and `#[route]` accept relative paths in the same way.

# Dynamic path parameters

To make a module's segment dynamic, call `path_param!` inside that module. The declaration names the URL parameter and can name a type to parse the segment into. The macro turns the module's segment into that parameter and generates a type, named in Pascal case, for reading it.

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
    view::{View, view},
};

path_param!(post_id: u64, error = bad_request);

#[page]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { <h1>"Post " (post_id)</h1> })
}
```

This page serves `/posts/{post_id}`. For a request to `/posts/42`, the segment `42` is parsed with `u64::from_str`. If parsing fails, the request gets `400 Bad Request`, because the declaration says `error = bad_request`.

The parameter name comes from `post_id` in the declaration, not from the file name. A file named `id.rs` would still add `{post_id}`.

What `path_param::<T>(cx)` returns depends on the declaration:

- After `path_param!(slug)`, `path_param::<Slug>(cx)` returns the percent-decoded segment as a `&str`. It cannot fail.
- After `path_param!(post_id: u64)`, `path_param::<PostId>(cx)` parses the segment with `FromStr` and returns `Result<&u64, &<u64 as FromStr>::Err>`.
- Adding `error = ...` with `bad_request`, `not_found`, `unauthorized`, `forbidden`, `redirect(...)`, or `redirect_permanent(...)` turns a parse failure into that router error.

The segment is parsed once per request. Later calls return the same memoized result.

A module adds one segment, so it can hold only one `path_param!`. Use nested modules for more parameters:

| Module | Route path |
|---|---|
| `app::organizations::organization_id` | `/organizations/{organization_id}` |
| `app::organizations::organization_id::users::user_id` | `/organizations/{organization_id}/users/{user_id}` |

Handlers and layouts in descendant modules can read the parameters of ancestor modules, as long as the generated types are visible to them.

# Catch-all parameters

Put `*` before the parameter name to make the module capture the rest of the path.

```rust
// src/app/docs/path.rs serves /docs/{*path}.
# use topcoat::router::path_param;
path_param!(*path);
```

This makes the module a `CatchAll` segment. It must be the last segment of the path, and it matches one or more segments.

Without a type, a handler reads the catch-all as [`CatchAllSegments`](crate::CatchAllSegments). With a segment type, it reads a slice of parsed values.

See the [`path_param!` reference](https://docs.rs/topcoat/latest/topcoat/router/macro.path_param.html) for the generated types, parsing, and errors.

A catch-all declared by hand with `segment!(kind = CatchAll)` can be read with [`raw_path_params`](crate::raw_path_params). Its value holds the encoded rest of the path, with the `/` separators, along with each segment decoded on its own.

# Query parameters

Query parameters do not affect module-derived paths. Declare a struct with named fields and `#[query_params]`, then read it from any handler that takes `cx: &Cx`.

```rust
# use topcoat::{
#     Result,
#     context::Cx,
#     router::{page, query_params},
#     view::{View, view},
# };
#[query_params(error = bad_request)]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

// In src/app/posts.rs, this serves /posts and accepts
// requests such as /posts?page=2&q=rust.
#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PostsQuery>(cx)?;
    Ok(view! {
        <p>"page: " (query.page.unwrap_or(1))</p>
        <p>"search: " (query.q.as_deref().unwrap_or(""))</p>
    })
}
```

`#[query_params]` derives `serde::Deserialize`. Use `Option<T>` for keys that may be missing. The query string is parsed once per request, and every call returns a reference to the same parsed struct.

# Segment overrides

`segment!(...)` changes the segment that its module adds to the path. It takes `kind` and `rename`, each at most once:

| Declaration | Result |
|---|---|
| none in `blog_posts` | `/blog-posts` |
| `segment!(rename = "articles")` | `/articles` |
| `segment!(kind = Group)` | no URL segment |
| `segment!(kind = Param, rename = "id")` | `/{id}` |
| `segment!(kind = CatchAll, rename = "path")` | `/{*path}` |

Regular modules are `Static` by default. Modules whose names start with `_` are `Group` by default. A rename is used exactly as written and is not converted to kebab-case.

`path_param!` already declares a `Param` or `CatchAll` segment, so do not use it together with `segment!` in the same module. A `Param` or `CatchAll` declared with `segment!` captures the segment, but does not generate a type for reading it.

# Groups

A module whose name starts with `_` is a group. It adds no segment to the URL, but it still counts when layouts and layers are matched to handlers.

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

Here the two groups apply different layouts to top-level URLs.

Set the kind explicitly when the module name should not decide it:

```rust
// src/app/marketing.rs: hide `marketing` from URLs.
topcoat::router::segment!(kind = Group);
```

```rust
// src/app/_internal.rs: serve the module at /internal.
topcoat::router::segment!(kind = Static);
```

A layout or layer in `_marketing` applies only to the handlers inside `_marketing`, even though the group name does not appear in request URLs.

# Explicit absolute paths

A `#[page]`, `#[layout]`, `#[layer]`, or `#[route]` with an absolute path string does not use the module tree. `segment!` declarations do not affect it, and `module_router!()` does not register it. Register it by name:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("/legacy")]
async fn legacy() -> Result<impl View> {
    Ok(view! { <h1>"Legacy"</h1> })
}

pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!()
        .page(legacy)
        .build()
}
```

To register all handlers with absolute paths at once, call `.discover()` on the builder instead, as shown in [Registering everything else](#registering-everything-else).

# Conflicts

Handlers in the same module share a path. They can serve different HTTP methods, but building the router panics when two of them serve the same method at the same path. A route for a specific method can share a path with a `*` route, and the specific route wins.

`module_router!` panics when two module-derived layouts, or two module-derived layers, have the same path. Link-time discovery does not give them a defined order. To run several layers at one path, register them yourself with `RouterBuilder::layer`, which runs them in a defined order.
