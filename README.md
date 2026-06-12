# Topcoat

> Early-stage and experimental. Expect breaking changes.

A batteries-included Rust web framework for server-rendered apps.

Topcoat sits on top of Axum and turns it into a productive full-stack toolkit: HTML-first templates, file-system-shaped routing, per-request memoization, and a built-in asset pipeline with optional Tailwind support — all designed so you can stay in Rust.

See the [Getting started guide](docs/getting_started.md) to set up a new project.

```rust,ignore
use topcoat::{Result, router::{Router, page}, view::{component, view}};

#[tokio::main]
async fn main() {
    topcoat::start(Router::new().discover()).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                hello(name: "World")
            </body>
        </html>
    }
}

#[component]
async fn hello(name: &str) -> Result {
    view! { <h1>"Hello, " (name) "!"</h1> }
}
```

## What makes Topcoat different

### HTML that's still HTML

The `view!` macro doesn't invent a Rust-shaped HTML dialect. Element names, attribute names, and void elements stay the way you'd write them in a `.html` file — `aria-label`, `hx-get`, `xmlns:xlink`, `<br>`, `<input>`, all of it. Control flow is just Rust:

```rust,ignore
view! {
    <ul>
        for post in posts {
            <li>
                <a href=(post.url) aria-current=(is_current.then_some("page"))>
                    (post.title)
                </a>
            </li>
        }
    </ul>
}
```

Attributes that evaluate to `false` or `None` drop themselves from the rendered HTML. Components are called with familiar function-call syntax and can take trailing child nodes.

### Your module tree is your route table

Drop `module_router!()` at the root of your `app` module and every `#[page]`, `#[layout]`, and `#[route]` below it gets registered automatically. Module names are kebab-cased into URL segments. Modules prefixed with `_` are *groups* — they hold shared layouts but don't add a segment.

```text
src/app/
├── mod.rs              → /            (and the root <html> layout)
├── about.rs            → /about
├── _marketing/
│   ├── mod.rs                         (layout, no segment)
│   └── pricing.rs      → /pricing
├── posts/
│   ├── mod.rs          → /posts
│   └── id/
│       └── mod.rs      → /posts/{post_id}
└── api/
    └── health.rs       → GET /api/health
```

### Functions, not middleware

Authentication, tenant lookup, feature flags, locale detection — anything request-scoped — is just a function that takes `&Cx`.

```rust,ignore
fn db(cx: &Cx) -> &Database {
    app_state(cx)
}

#[memoize]
async fn fetch_user(cx: &Cx, id: &str) -> Option<User> {
    db(cx).load_user(id).await
}

async fn require_auth(cx: &Cx) -> Result<&User, UnauthorizedError> {
    let id = session_cookie(cx).ok_or_unauthorized()?;
    fetch_user(cx, id).await.ok_or_unauthorized()
}

#[component]
async fn user_avatar(cx: &Cx) -> Result {
    let user = require_auth(cx).await?;
    view! { <img src=(user.avatar_url) alt=(format!("{}'s avatar", user.name))> }
}
```

`#[memoize]` caches per request and keys on the arguments — so a layout reading the current user, a page checking authorization, and a deep component rendering an avatar all share one database hit. Concurrent callers even await the same in-flight future.

### Asset bundling

```rust,ignore
const FERRIS: Asset = asset!("./ferris.png");

view! { <img src=(FERRIS)> }
```

The bundler scans your compiled binary for `asset!` calls, copies (or downloads) every file, and serves them at `/_topcoat/assets/ferris-<hash>.png`. Remote assets can be pinned with a SHA-256 checksum.

Tailwind is the same story without a `node_modules`: enable the `tailwind` feature, drop a `build.rs` one-liner, and the standalone Tailwind CLI's output becomes a normal Topcoat asset.

```rust,ignore
view! { <link rel="stylesheet" href=(tailwind::stylesheet!())> }
```

### The CLI

- `topcoat dev` — rebuild, rebundle assets, restart the app on changes.
- `topcoat fmt` — format the inside of `view!` so it reads cleanly next to `rustfmt`.
- `topcoat asset` — produce the asset bundle for release builds.

## Learn Topcoat

**Start here**
- [Getting started](docs/getting_started.md) — create a new project, install the CLI, run the dev server.

**Rendering**
- [The `view!` macro](docs/view.md) — templating syntax, control flow, conditional attributes.
- [The `component` macro](docs/component.md) — async functions as components, with child content.
- [The `attributes!` macro](docs/attributes.md) — reusable runtime attribute fragments.

**Routing**
- [Router](docs/router.md) — pages, layouts, and API routes; manual and auto-discovered.
- [Module-based routing](docs/module_router.md) — derive the route table from your module tree.

**Working with requests**
- [Request context (`Cx`)](docs/context.md) — the value pages, layouts, and components read from.
- [App state](docs/app_state.md) — share long-lived values across requests, keyed by type.
- [Path and query params](docs/path_and_query_params.md) — typed `T::of(cx)` accessors.
- [Request and response bodies](docs/request_response.md) — JSON, forms, custom extractors and responses.

**Patterns**
- [Functions, not middlewares](docs/functions_not_middlewares.md) — the recommended way to model auth and other request-scoped concerns.
- [Memoization](docs/memoization.md) — `#[memoize]` for per-request caching and fan-out dedup.

**Project infrastructure**
- [Assets](docs/assets.md) — declare assets in Rust, serve them with content-hashed URLs.
- [Tailwind](docs/tailwind.md) — Tailwind CSS without Node, wired into the asset pipeline.
- [Source code formatting](docs/source_formatting.md) — `topcoat fmt` for macro bodies.
