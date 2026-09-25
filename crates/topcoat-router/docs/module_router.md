The `module_router!` macro derives a handler's path from its enclosing Rust module. A handler without a path string uses the module path. A handler whose path string starts with `./` is served below the module path. Absolute path strings are ignored by the module router entirely.

# Setup

Call `module_router!()` from the root module of the route tree. That module maps to `/`. The macro returns a `RouterBuilder`, so add everything else your application needs on that builder before calling `.build()`.

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

# Registering everything else

`module_router!()` registers module-derived handlers. Register other handlers and application services on the returned builder, just as you would with `Router::builder()`.

Call `RouterBuilderDiscoverExt::discover` to add explicit-path handlers and other items collected through discovery:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    topcoat::router::module_router!().discover().build()
}
```

Anything that is registered as a value is passed in by hand. The asset bundle is the common case: a view that renders an `Asset`, such as a Tailwind stylesheet or a self-hosted font, needs the bundle on the router.

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

Some integrations require registered application values. A missing value may cause a panic when a request first uses it. Follow the integration's setup guide when adding it to the router.

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

A relative path string starting with `./` is joined onto the module path. This places a handler below its module without adding a module for it.

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

This can also be used to add a trailing slash to the end of your module path. A bare `./` serves the module path itself with a trailing slash, and a relative path ending in a slash keeps it.

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

The same form works for `#[layout]`, `#[layer]`, and `#[route]`.

For parameters written in a relative path, add `segment = false` to each `path_param!` declaration. This defines the typed accessor without changing the module's segment.

```rust
// src/app/posts.rs: GET /posts/{post_id}
# use topcoat::{Result, context::Cx, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64, segment = false, error = bad_request);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { <h1>"Post " (post_id)</h1> })
}
```

Several parameters can be declared this way in one module. For example, a handler in `app::users` with the path `./{user_id}/posts/{post_id}/comments/{comment_id}` serves `/users/{user_id}/posts/{post_id}/comments/{comment_id}` when all three declarations use `segment = false`. The option also works for catch-all parameters and can be combined with a manual `segment!` override.

# Dynamic path parameters

Call `path_param!` inside the module that should become dynamic. The declaration names the URL parameter and may name the type used to parse each captured segment. The macro changes that module's segment to the parameter and generates the Pascal-cased type used to read it.

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

This page serves `/posts/{post_id}`. A request for `/posts/42` parses `42` with `u64::from_str`. A failed parse returns `400 Bad Request` because the declaration uses `error = bad_request`.

The parameter name comes from `post_id` in the declaration, not from the filename. The file could be named `id.rs` and would still contribute `{post_id}`.

`path_param::<T>(cx)` returns a request-scoped value:

- After `path_param!(slug)`, `path_param::<Slug>(cx)` returns the percent-decoded segment as `&str` and cannot fail.
- After `path_param!(post_id: u64)`, `path_param::<PostId>(cx)` parses with `FromStr`. Without `error = ...`, the function returns `Result<&u64, &<u64 as FromStr>::Err>`.
- An `error = ...` option maps a parse failure to a router error. See the [`path_param!` reference](https://docs.rs/topcoat/latest/topcoat/router/macro.path_param.html) for the supported forms.

Parsing occurs once per request. Later calls return the memoized result.

A module contributes one segment, so only one `path_param!` per module can use the default `segment = true`. Use nested modules for multiple module-derived parameters, or `segment = false` for parameters in relative paths as shown above:

| Module | Route path |
|---|---|
| `app::organizations::organization_id` | `/organizations/{organization_id}` |
| `app::organizations::organization_id::users::user_id` | `/organizations/{organization_id}/users/{user_id}` |

Handlers and layouts in descendant modules can read parameters declared by ancestor modules if the Rust types are visible there.

# Catch-all parameters

Prefix a parameter name with `*` when its module should capture the remaining path.

```rust
// src/app/docs/path.rs contributes /docs/{*path}.
# use topcoat::router::path_param;
path_param!(*path);
```

The declaration emits a `CatchAll` segment override. The module must be the last served segment, and the catch-all matches at least one segment.

Handlers read an unparsed catch-all as [`CatchAllSegments`](crate::CatchAllSegments) or add a segment type to read a parsed slice.

See the [`path_param!` macro reference](https://docs.rs/topcoat/latest/topcoat/router/macro.path_param.html) for value shapes, construction, parsing, and errors.

A manual `segment!(kind = CatchAll)` capture remains available through [`raw_path_params`](crate::raw_path_params). Its value carries the encoded tail, with `/` separators intact, next to its separately decoded segments.

# Query parameters

Query parameters do not affect module-derived paths. Declare a named-field struct with `#[query_params]`, then read it from any handler that takes `cx: &Cx`.

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

By default, `path_param!` emits a `Param` or `CatchAll` segment override. To combine it with `segment!` in the same module, set `segment = false` on the parameter declaration. A manual override creates the route capture but does not define a typed accessor.

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

# Explicit absolute paths

Adding an absolute path string to `#[page]`, `#[layout]`, `#[layer]`, or `#[route]` disables module path derivation for that item. `segment!` declarations do not affect explicit absolute paths.

`module_router!()` discovers module-derived handlers. Register an absolute-path handler by name:

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

To register all explicit-path handlers at once, call `.discover()` on the returned builder instead, as described in "Registering everything else".

# Conflicts

Handlers in one module share a derived path. They may serve different HTTP methods, but overlapping methods at the same served path are rejected when the router is built. A specific-method route may share a path with a `*` route and takes precedence.

`module_router!` rejects two module-derived layouts or two module-derived layers at the same logical path because link-time discovery does not define their order. To run several layers at one path, register them explicitly with `RouterBuilder::layer`, which gives their order meaning.
