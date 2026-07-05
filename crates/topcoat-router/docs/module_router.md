The `module_router!` macro derives routes from your Rust module structure. The module tree becomes the route table: page, layout, layer, and route handlers can omit explicit path strings and let their enclosing module decide the URL.

# Setup

Call `module_router!()` from the root module of your route tree. This module becomes the root `/` path. The macro returns a `RouterBuilder`, so call `.build()` once you have added anything else the builder needs.

```rust
// src/app.rs
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!().build()
}
```

Every module-derived `#[page]`, `#[layout]`, `#[layer]`, and `#[route]` under `app` is discovered and registered.

# How modules map to routes

Each module's path relative to the root module determines its URL. Module names are converted to **kebab-case** (e.g. `user_settings` becomes `user-settings`).

| Module | Route |
|---|---|
| `app` | `/` |
| `app::about` | `/about` |
| `app::blog_posts` | `/blog-posts` |
| `app::settings` | `/settings` |
| `app::settings::profile` | `/settings/profile` |

# Pages, layouts, layers, and API routes

A `#[page]` defines a page handler. A `#[layout]` wraps all pages in the same module and its submodules.

```rust
# use topcoat::{Result, router::{Slot, layout, page}, view::view};
// src/app.rs: layout at "/" wraps all pages
#[layout]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <html><body>(slot.await?)</body></html>
    }
}

#[page]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}
```

```rust
# use topcoat::{Result, router::page, view::view};
// src/app/about.rs: page at "/about"
#[page]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
```

API routes use `#[route]` with an explicit HTTP method. Like pages and layouts, method-only routes derive their URL from the module path:

```rust
# use topcoat::{Result, router::route};
// src/app/api/health.rs: GET /api/health
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

Layers use the same module-derived path as layouts, but wrap request handling instead of rendered view output:

```rust
// src/app/api.rs: wraps routes under /api
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, Response, layer},
};

#[layer]
async fn api_log(cx: &mut Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```

# Path overrides

Module-derived paths and explicit paths can be mixed in the same route tree. `#[page]`, `#[layout]`, `#[layer]`, and `#[route]` all register into the same builder in the end. If an attribute includes an explicit path, that path is used instead of the module-derived path for that item:

```rust
# use topcoat::{
#     Result,
#     context::Cx,
#     router::{Body, Next, Response, Slot, layer, layout, page, route},
#     view::view,
# };
#[page("/")]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}

#[layout("/admin")]
async fn admin_layout(slot: Slot<'_>) -> Result {
    view! { <main>(slot.await?)</main> }
}

#[layer("/admin")]
async fn admin_layer(cx: &mut Cx, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

# Renaming a static segment

`segment!(rename = "name")` overrides the URL with the given literal (used as-is, no kebab-casing).

```rust
// src/app/blog_post.rs
topcoat::router::segment!(rename = "articles");
// Route: /articles instead of /blog-post
```

# Groups

Modules prefixed with `_` are **groups**. They organize code and can hold shared layouts or layers, but they do not add a path segment to the served URL.

```text
app.rs                 # layout at /
app/
  _marketing.rs        # layout wrapping marketing pages (no URL segment)
  _marketing/
    pricing.rs         # /pricing
    features.rs        # /features
  _docs.rs             # layout wrapping docs pages (no URL segment)
  _docs/
    getting_started.rs # /getting-started
```

Both `pricing` and `getting_started` are top-level routes, but they can have different layouts through their respective group module files.

You can also turn a regular module into a group with `segment!(kind = Group)`:

```rust
// src/app/marketing.rs
topcoat::router::segment!(kind = Group);
// `marketing` now contributes no URL segment.
```

Or turn a group module into a regular static path segment:

```rust
// src/app/_group.rs
topcoat::router::segment!(kind = Static);
// Module now reachable as `/group`.
```

The `_` prefix can also act as a naming convention for route-specific utilities. For example, a `_components` module for shared UI fragments:

```text
app.rs
app/
  _components.rs       # exports shared components, no route
  _components/
    header.rs
    footer.rs
  about.rs             # /about: can use app::_components::header
  contact.rs           # /contact
```
