# Topcoat

> Very early-stage and experimental. Expect breaking changes.

A batteries-included web framework for building server-rendered web apps in Rust.

Topcoat takes the opposite bet from Leptos and Dioxus: instead of running Rust in the browser via WASM, it keeps all rendering on the server and uses [HTMX](https://htmx.org) for interactivity. The result is minimal JavaScript, no hydration overhead, and a simple mental model — the server owns the state.

Built on top of [Axum](https://github.com/tokio-rs/axum).

## Features

- **Optional module-based router** — routes are derived from your module structure, no registration boilerplate; or register routes manually if you prefer
- **`view!` macro** — write HTML that looks like HTML, with Rust control flow (`if`, `match`, `for`, `let`)
- **No surprising HTML** — void elements stay void, no self-closing components, no camelCase attributes
- **Layouts** — wrap pages in shared layouts via a `Slot` composition model
- **Components** — reusable async functions that render to a `View`
- **Dev server** — `topcoat dev` watches for changes and hot-reloads the browser
- **Axum compatible** — you can drop down to raw Axum when needed

## Quick start

```toml
# Cargo.toml
[dependencies]
topcoat = "0.1"
tokio = { version = "1", features = ["full"] }
```

```
src/
  main.rs
  app/
    mod.rs       # layout + home page
    about.rs     # /about
    _group/
      mod.rs
      contact.rs # /contact  (group prefix _ is stripped from the path)
```

**`src/main.rs`**
```rust
mod app;

#[tokio::main]
async fn main() {
    let router = app::router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::serve(listener, router).await.unwrap();
}
```

**`src/app/mod.rs`**
```rust
mod _group;
mod about;

use topcoat::{
    router::{Result, Slot, layout, page},
    view::view,
};

pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!()
}

#[layout]
async fn layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"hello world"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    <span>" | "</span>
                    <a href="/about">"about"</a>
                </nav>
                <hr>
                (slot.await?)
            </body>
        </html>
    }
}

#[page]
async fn home_page() -> Result {
    view! { "home" }
}
```

**`src/app/about.rs`**
```rust
use topcoat::{router::{Result, page}, view::view};

#[page]
async fn about_page() -> Result {
    view! { "about" }
}
```

## Module-based routing

Routes are derived automatically from your module structure. The rules are:

| Module | Route |
|---|---|
| `app` | `/` |
| `app::about` | `/about` |
| `app::settings` | `/settings` |
| `app::settings::profile` | `/settings/profile` |
| `app::_group::contact` | `/contact` |
| `app::_group` | *(group root, no route)* |

Modules prefixed with `_` are **groups** — they organize code without adding a path segment.

## Manual routing

If you prefer not to use the module router, you can register pages and layouts explicitly. Annotate each function with an explicit path and register them on the router directly:

```rust
use topcoat::{
    router::{Result, Router, Slot, layout, page},
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <html><body>(slot.await?)</body></html>
    }
}

#[page("/")]
async fn home_page() -> Result {
    view! { "home" }
}

#[page("/about")]
async fn about_page() -> Result {
    view! { "about" }
}

pub fn router() -> Router {
    Router::new()
        .layout(root_layout)
        .page(home_page)
        .page(about_page)
}
```

## The `view!` macro

The `view!` macro is a templating system that aims to be close to real HTML with no surprises:

- Elements use their real HTML names
- Void elements (`<hr>`, `<br>`, `<input>`, etc.) do not need a closing tag
- Components use a function-call syntax with named parameters `component_name(name: value, <div>"foo"</div>)`
- Rust expressions are wrapped in `()`
- String literals must be quoted

```rust
view! {
    <div class="container">
        <h1>"Hello, " (name) "!"</h1>

        if logged_in {
            <a href="/dashboard">"Go to dashboard"</a>
        } else {
            <a href="/login">"Log in"</a>
        }

        <ul>
            for item in &items {
                <li>(item)</li>
            }
        </ul>

        <input type="text" value=(default_value)>
        <hr>
    </div>
}
```

## Components

Components are async functions annotated with `#[component]`. They receive typed parameters including child `View`s.

```rust
use topcoat::{component, router::Result, view::{View, view}};

#[component]
async fn button<'a>(id: &'a str, child: View) -> Result {
    view! {
        <button id=(id) class="button">(child)</button>
    }
}
```

Components are invoked with function-call syntax inside `view!`. Named arguments use `name: value` and any other positional arguments are appended to the component's child `View`:

```rust
view! {
    button(id: "submit", "Click me")
}
```

## Layouts

A `#[layout]` wraps all pages found in the same module (and submodules). It receives a `Slot` — a future that resolves to the page's rendered output.

```rust
#[layout]
async fn layout(slot: Slot<'_>) -> Result {
    view! {
        <html>
            <body>
                (slot.await?)
            </body>
        </html>
    }
}
```

## Dev server

```sh
topcoat dev
# or
cargo topcoat dev
```

Topcoat watches your source files, rebuilds on changes, and sends a reload signal to the browser. The `topcoat::dev::script()` component in your layout handles the client side — it's a no-op in production.

## Architecture

Topcoat is built on top of Axum, adding module-based routing, server-side templating, and a component model on top of Axum's solid HTTP foundation. `topcoat::router::Router` is convertible to `axum::Router` if you need to drop down to raw Axum.

## Planned

- **Component library** — an official UI component library built on Topcoat and Tailwind, in the spirit of shadcn/ui but designed for server-rendered HTMX apps: copy-paste components that live in your codebase, styled with Tailwind, interactive via HTMX without any client-side JavaScript framework
- **Server actions** — bring form submissions and mutations into the same server-side model as pages, without writing explicit API endpoints (similar to Next.js server actions)
- **Batteries included auth** — a built-in authentication system covering the common cases out of the box
- **Request-level memoization** — deduplicate repeated calls to the same data-fetching function within a single request, similar to React's `cache()`
- **Request hooks** — a request context that allows defining hooks like `use_auth()` which fetch data on demand (e.g. the current user from the database) and can be called from any component without threading the value down through every layer of the tree
- **Partial page re-rendering** — leverage HTMX to swap only the content that changed during navigation, so shared elements like the nav bar are not re-fetched or re-rendered on every page transition
- **Strong security model** — safe defaults with no surprises; things like CSRF protection, output escaping, and secure session handling should work correctly out of the box without requiring explicit opt-in
- **`view!` formatter** — source code formatting for the `view!` macro via `topcoat fmt`
