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
async fn root_layout(slot: Slot) -> View {
    view! {
        <html><body>(slot.await)</body></html>
    }
}

#[page]
async fn home() -> View {
    view! { <h1>"Home"</h1> }
}
```

```rust
// src/app/about.rs — page at "/about"
#[page]
async fn about() -> View {
    view! { <h1>"About"</h1> }
}
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

You can also turn a regular module into a group with `segment!(_)`:

```rust
// src/app/marketing/mod.rs
topcoat::router::segment!(_);
// `marketing` now contributes no URL segment.
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

## Renaming a static segment

`segment!("name")` overrides the URL with the given literal (used as-is, no kebab-casing) and forces the kind to `Static`.

```rust
// src/app/blog_post.rs
topcoat::router::segment!("articles");
// Route: /articles instead of /blog-post
```

This also works to flip a `_`-prefixed group into a regular static segment:

```rust
// src/app/_internal/mod.rs
topcoat::router::segment!("internal");
// Route: /internal instead of being a hidden group
```

## Dynamic segments (params)

`segment!(name)` marks the module as a dynamic parameter and generates an accessor function with the same name.

```rust
// src/app/users/id/mod.rs
topcoat::router::segment!(id);
```

This maps the `app::users::id` module to `/users/{id}` and generates an accessor:

```rust
fn id(cx: &Cx) -> &str { /* … */ }
```

You can read the captured value from any handler in this module or its submodules:

```rust
#[page]
async fn user_profile(cx: &Cx) -> View {
    view! { <h1>"User: " (id(cx)) </h1> }
}
```

Any submodule inherits the param segment:

| Module | Route |
|---|---|
| `app::users::id` | `/users/{id}` |
| `app::users::id::settings` | `/users/{id}/settings` |

### Typed params

Append `: Type` to parse the captured string into a custom type. The type must implement `FromStr`.

```rust
// src/app/posts/id/mod.rs
topcoat::router::segment!(id: uuid::Uuid);
```

The accessor now returns the parsed value:

```rust
fn id(cx: &Cx) -> uuid::Uuid { /* … */ }
```

### Renaming the accessor

If the URL param name and the accessor name should differ, append `as <fn_name>`:

```rust
// src/app/posts/id/mod.rs
topcoat::router::segment!(id: uuid::Uuid as param);
```

This still routes as `/posts/{id}` but the accessor is `param(cx) -> uuid::Uuid`.

## Catch-all segments

`segment!(..name)` matches all remaining path segments:

```rust
// src/app/docs/path/mod.rs
topcoat::router::segment!(..path);
```

This maps the `app::docs::path` module to `/docs/{*path}`.

## Segment forms at a glance

| Form | Kind | URL | Generated accessor |
|---|---|---|---|
| `segment!("blog-posts")` | `Static` | `/blog-posts` | — |
| `segment!(_)` | `Group` | *(hidden)* | — |
| `segment!(post_id)` | `Param` | `/{post_id}` | `post_id(cx) -> &str` |
| `segment!(post_id: uuid::Uuid)` | `Param` | `/{post_id}` | `post_id(cx) -> uuid::Uuid` |
| `segment!(post_id as my_post_id)` | `Param` | `/{post_id}` | `my_post_id(cx) -> &str` |
| `segment!(..rest)` | `CatchAll` | `/{*rest}` | — |
