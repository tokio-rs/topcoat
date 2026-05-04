# Router

`topcoat::router::Router` is the core routing primitive. It collects pages and layouts, matches layouts to pages by path prefix, and converts into an `axum::Router` for serving.

You can register pages and layouts in two ways: **manually** (explicit paths, full control) or with **auto-discovery** (the `discover` feature collects annotated items automatically). The [module router](./module_router.md) builds on top of both — this document covers using `Router` directly.

## Pages

A page is an async function annotated with `#[page]` and an explicit path:

```rust
use topcoat::{router::{Result, page}, view::view};

#[page("/")]
async fn home() -> Result {
    view! { <h1>"Home"</h1> }
}

#[page("/about")]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
```

The path string uses Axum's routing syntax — static segments, `{param}` for dynamic parameters, and `{*catch_all}` for wildcard tails:

```rust
#[page("/users/{id}")]
async fn user_profile() -> Result {
    view! { <h1>"User profile"</h1> }
}

#[page("/docs/{*path}")]
async fn docs_page() -> Result {
    view! { <h1>"Documentation"</h1> }
}
```

## Layouts

A layout wraps pages. It receives a `Slot` — a future that resolves to the inner page's rendered output. Annotate it with `#[layout]` and an explicit path:

```rust
use topcoat::{
    router::{Result, Slot, layout},
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Slot) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/about">"About"</a>
                </nav>
                (slot.await?)
            </body>
        </html>
    }
}
```

A layout applies to every page whose path starts with the layout's path. A layout at `"/"` wraps all pages. A layout at `"/settings"` wraps `/settings`, `/settings/profile`, `/settings/billing`, etc.

### Nested layouts

When multiple layouts match a page, they nest from innermost (most specific path) to outermost (least specific):

```rust
#[layout("/")]
async fn root_layout(slot: Slot) -> Result {
    view! { <html><body>(slot.await?)</body></html> }
}

#[layout("/settings")]
async fn settings_layout(slot: Slot) -> Result {
    view! {
        <div class="settings-shell">
            <nav>"Settings nav"</nav>
            (slot.await?)
        </div>
    }
}

#[page("/settings/profile")]
async fn profile() -> Result {
    view! { <h1>"Profile"</h1> }
}
```

A request to `/settings/profile` renders: `root_layout` > `settings_layout` > `profile`.

## Manual registration

Build a router by chaining `.page()` and `.layout()`:

```rust
use topcoat::router::Router;

pub fn router() -> Router {
    Router::new()
        .layout(root_layout)
        .layout(settings_layout)
        .page(home)
        .page(about)
        .page(profile)
}
```

Order doesn't matter — layout-to-page matching is based on path prefixes, not registration order.

## Auto-discovery with `discover()`

With the `discover` feature enabled, every `#[page]` and `#[layout]` is automatically collected at link time. Instead of listing each item by hand, call `.discover()`:

```rust
pub fn router() -> Router {
    Router::new().discover()
}
```

This finds all pages and layouts across your entire crate (and dependencies) and registers them.

## Serving

Use `topcoat::serve` to run it:

```rust
#[tokio::main]
async fn main() {
    let router = my_app::router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::serve(listener, router).await.unwrap();
}
```

You can also convert to `axum::Router` directly if you need to add Axum middleware, merge with other Axum routers, or use Axum's `serve` directly:

```rust
let axum_router: axum::Router = router.into();
```

## Example: full manual setup

```rust
use topcoat::{
    router::{Result, Router, Slot, layout, page},
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Slot) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav>
                    <a href="/">"Home"</a>
                    <a href="/users">"Users"</a>
                </nav>
                (slot.await?)
            </body>
        </html>
    }
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

pub fn router() -> Router {
    Router::new()
        .layout(root_layout)
        .page(home)
        .page(users_list)
        .page(user_profile)
}
```

## Example: same app with `discover()`

```rust
// The page and layout definitions are identical — only the router function changes.

pub fn router() -> Router {
    Router::new().discover()
}
```

All `#[page]` and `#[layout]` items from the example above (and any other module in the crate) are picked up automatically.
