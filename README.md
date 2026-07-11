# Topcoat

> Early-stage and experimental. Expect breaking changes.

A modular, batteries-included Rust web framework for server-rendered apps.

See the [Getting started guide](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md) to set up a new project.

```rust,ignore
use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build()).await.unwrap();
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

### Powerful, unsurprising HTML templates

The `view!` macro stays true to HTML and Rust. Use familiar Rust control flow as part of your templates:

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

Use the `topcoat fmt` CLI command to automatically format `view!` snippets across your codebase.

### Module-based routing

Topcoat can optionally infer your route tree from your app's module structure (without a build step):

```text
src/
|-- app.rs              -> /            (and the root <html> layout)
`-- app/
    |-- about.rs        -> /about
    |-- _marketing.rs                  (layout, no URL segment)
    |-- _marketing/
    |   `-- pricing.rs  -> /pricing
    |-- posts.rs        -> /posts
    |-- posts/
    |   `-- id.rs       -> /posts/{post_id}
    `-- api/
        `-- health.rs   -> GET /api/health
```

### Asset bundling

The bundler scans your compiled binary for `asset!` calls, copies (or even downloads) every file into a local asset directory, and allows Topcoat to serve them efficiently with aggressive browser caching.

```rust,ignore
const FERRIS: Asset = asset!("./ferris.png");

view! { <img src=(FERRIS)> }
```


### Built-in Tailwind support

Enabled the `tailwind` feature to integrate Tailwind into your project effortlessly:

```rust,ignore
view! { <link rel="stylesheet" href=(topcoat::tailwind::stylesheet!())> }
```

## Learn Topcoat

**Start here**
- [Getting started](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md): create a new project, install the CLI, run the dev server.
- [Source code formatting](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-cli/docs/fmt.md): `topcoat fmt` for macro bodies.

**Rendering**
- [The `view!` macro](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-view/macro/docs/view.md): templating syntax, control flow, conditional attributes.
- [The `#[component]` macro](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-view/macro/docs/component.md): async functions as components, with child content.
- [The `attributes!` macro](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-view/macro/docs/attributes.md): reusable runtime attribute fragments.
- [The `class!` macro](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-view/macro/docs/class.md): space-separated class lists from static and conditional entries.

**Routing**
- [Router](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/router.md): pages, layouts, and API routes; manual and auto-discovered.
- [Module-based routing](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-router/docs/module_router.md): derive the route table from your module tree.

**Working with requests**
- [Request context (`Cx`)](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/context.md): the value pages, layouts, and components read from.
- [App context](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/app_context.md): share long-lived values across requests, keyed by type.
- [Cookies](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/cookie.md): read and write the request cookie jar, with signed, encrypted, and prefixed cookies.
- [Memoization](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-core/macro/docs/memoization.md): `#[memoize]` for per-request caching and fan-out dedup.
- [Functions, not middlewares](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/functions_not_middlewares.md): the recommended way to model auth and other request-scoped concerns.

**Asset system**
- [Assets](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/asset.md): declare assets in Rust, serve them with content-hashed URLs.
- [Fonts](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/font.md): bundle and serve web fonts.
- [Icons](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/icon.md): download Iconify icon sets or declare your own.

**Third-party integrations**
- [Tailwind](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/tailwind.md): Tailwind CSS without Node, wired into the asset pipeline.
- [htmx](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/htmx.md): drive partial HTML swaps from the server with request/response header helpers.
