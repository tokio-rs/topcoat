# Module-based routing

The `module_router!` macro derives routes from your Rust module structure. No manual route registration, no path strings scattered across files — your module tree *is* your route table.

## Setup

Call `module_router!()` from the root module of your route tree. This module becomes the root `/` path.

```rust
// src/app/mod.rs
pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!()
}
```

Every `#[page]` and `#[layout]` in modules under `app` is automatically discovered and registered.

## How modules map to routes

Each module's path relative to the root module determines its URL. Module names are converted to **kebab-case** (`user_settings` becomes `user-settings`).

| Module | Route |
|---|---|
| `app` | `/` |
| `app::about` | `/about` |
| `app::blog_posts` | `/blog-posts` |
| `app::settings` | `/settings` |
| `app::settings::profile` | `/settings/profile` |

## Pages and layouts

A `#[page]` defines a route handler. A `#[layout]` wraps all pages in the same module and its submodules.

```rust
// src/app/mod.rs — layout at "/" wraps all pages
#[layout]
async fn root_layout(slot: Slot) -> Result {
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
// src/app/about.rs — page at "/about"
#[page]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
```

## Renaming a static segment

`segment!(rename = "name")` overrides the URL with the given literal (used as-is, no kebab-casing).

```rust
// src/app/blog_post.rs
topcoat::router::segment!(rename = "articles");
// Route: /articles instead of /blog-post
```

## Groups

Modules prefixed with `_` are **groups**. They organize code and can hold shared layouts, but they don't add a path segment to the URL.

```
app/
  mod.rs              # layout at /
  _marketing/
    mod.rs            # layout wrapping marketing pages (no route segment)
    pricing.rs        # /pricing
    features.rs       # /features
  _docs/
    mod.rs            # layout wrapping docs pages (no route segment)
    getting_started.rs # /getting-started
```

Both `pricing` and `getting_started` are top-level routes, but they can have different layouts via their respective group `mod.rs` files.

You can also turn a regular module into a group with `segment!(kind = Group)`:

```rust
// src/app/marketing/mod.rs
topcoat::router::segment!(kind = Group);
// `marketing` now contributes no URL segment.
```

Or turn a group module into a regular static path segment:

```rust
// src/app/_group/mod.rs
topcoat::router::segment!(kind = Static);
// Module now reachable as `/group`.
```

The `_`-prefix can also act as a naming convention for route-specific utilities. For example, a `_components` module for shared UI fragments:

```
app/
  mod.rs
  _components/
    mod.rs            # exports shared components, no route
    header.rs
    footer.rs
  about.rs            # /about — can use app::_components::header
  contact.rs          # /contact
```
