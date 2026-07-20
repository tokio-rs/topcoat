<div align="center">
  <h1>Topcoat</h1>
</div>

<div align="center">
  <h3>The full full-stack framework for Rust</h3>
</div>

<div align="center">

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

</div>

[crates-badge]: https://img.shields.io/crates/v/topcoat.svg?style=flat-square
[crates-url]: https://crates.io/crates/topcoat
[docs-badge]: https://img.shields.io/docsrs/topcoat?style=flat-square
[docs-url]: https://docs.rs/topcoat/latest/topcoat
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square
[mit-url]: https://github.com/tokio-rs/topcoat/blob/main/LICENSE
[actions-badge]: https://img.shields.io/github/actions/workflow/status/tokio-rs/topcoat/ci.yml?branch=main&style=flat-square
[actions-url]: https://github.com/tokio-rs/topcoat/actions?query=workflow%3ACI+branch%3Amain
[discord-badge]: https://img.shields.io/discord/500028886025895936.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/tokio

Topcoat is a modular, batteries-included Rust framework for building fullstack apps. It prioritizes simplicity and productivity. See [Learn Topcoat](#learn-topcoat) to get started, or the [Roadmap](#roadmap) for what's coming next.

**Early-stage and experimental. Expect breaking changes.**

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

### Client reactivity without the boilerplate

Topcoat renders all markup on the server: components can be async and query the database directly, eliminating all the traditional boilerplate needed for a separate API layer. Interactivity does not have to cost a round-trip, though. A `$(...)` expression is ordinary type-checked Rust that Topcoat evaluates on the server for the initial render and also translates to JavaScript, so it re-runs instantly in the browser. No wasm bundle, no client build step:

```rust,ignore
view! {
    signal open = false;

    // Runs entirely in the browser; no server round-trip.
    <button @click=$(|_e| open.set(!open.get()))>"What is Topcoat?"</button>
    <p :hidden=$(!open.get())>"A fullstack Rust framework."</p>
}
```

When an update does need the server, like fresh search results, mark the component as a `#[shard]`. Topcoat re-renders it on the server whenever one of its `$(...)` arguments changes and swaps the new HTML in place:

```rust,ignore
#[component]
async fn search() -> Result {
    view! {
        signal query = String::new();

        <input @input=$(|e: Event| query.set(e.target.value))>

        // Updates as the user types.
        search_results(query: $(query.get()))
    }
}

#[shard]
async fn search_results(cx: &Cx, query: String) -> Result {
    view! {
        <ul>
            // Your own server-side code, like a database query:
            for product in search_products(cx, &query).await? {
                <li>(product.name)</li>
            }
        </ul>
    }
}
```

### Powerful, unsurprising HTML templates

The `view!` macro stays true to HTML and Rust. Use familiar Rust control flow as part of your templates:

```rust,ignore
view! {
    <nav>
        for item in nav_items {
            <a
                href=(item.url)
                if item.url == current_path {
                    aria-current="page"
                    class="active"
                }
            >
                (item.label)
            </a>
        }
    </nav>
}
```

Use the `topcoat fmt` CLI command to automatically format `view!` snippets (and other macros) across your codebase.

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

### Premade components you can edit

Topcoat UI is a component library based on [Tailwind](https://tailwindcss.com/) inspired by [shadcn/ui](https://ui.shadcn.com/). Components are copied into your project via the `topcoat ui` CLI command, meaning you can freely change their design and functionality to fit your use case:

```rust,ignore
#[component]
async fn delete_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Delete workspace")
                card_description(
                    "This permanently removes the workspace and all of its data."
                )
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                button(variant: ButtonVariant::Ghost, "Cancel")
                button(variant: ButtonVariant::Destructive, "Delete workspace")
            )
        )
    }
}
```

### Asset bundling

The bundler scans your compiled binary for `asset!` calls, copies (or even downloads) every file into a local asset directory, and allows Topcoat to serve them efficiently with aggressive browser caching.

```rust,ignore
const FERRIS: Asset = asset!("./ferris.png");

view! { <img src=(FERRIS)> }
```

Topcoat also ships with utilities for web fonts and icons, as well as easy integrations for [Fontsource](https://fontsource.org/) (Google Fonts) and [Iconify](https://icon-sets.iconify.design/).


### Built-in Tailwind support

Enabled the `tailwind` feature to integrate [Tailwind](https://tailwindcss.com/) into your project effortlessly:

```rust,ignore
view! { <link rel="stylesheet" href=(topcoat::tailwind::stylesheet!())> }
```

## Learn Topcoat

**Start here**
- [Getting started](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md): create a new project, install the CLI, run the dev server.
- [Source code formatting](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat-cli/docs/fmt.md): `topcoat fmt` for macro bodies.

**Rendering**
- [The `view!` macro](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html): templating syntax, control flow, conditional attributes.
- [The `#[component]` macro](https://docs.rs/topcoat/latest/topcoat/view/attr.component.html): async functions as components, with child content.
- [The `attributes!` macro](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html): reusable runtime attribute fragments.
- [The `class!` macro](https://docs.rs/topcoat/latest/topcoat/view/macro.class.html): space-separated class lists from static and conditional entries.

**Routing**
- [Router](https://docs.rs/topcoat/latest/topcoat/router/index.html): pages, layouts, and API routes; manual and auto-discovered.
- [Module-based routing](https://docs.rs/topcoat/latest/topcoat/router/macro.module_router.html): derive the route table from your module tree.

**Working with requests**
- [Request context (`Cx`)](https://docs.rs/topcoat/latest/topcoat/context/index.html): the value pages, layouts, and components read from.
- [App context](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/app_context.md): share long-lived values across requests, keyed by type.
- [Memoization](https://docs.rs/topcoat/latest/topcoat/context/attr.memoize.html): `#[memoize]` for per-request caching and fan-out dedup.
- [Functions, not middlewares](https://docs.rs/topcoat/latest/topcoat/context/index.html#functions-not-middlewares): the recommended way to model auth and other request-scoped concerns.
- [Cookies](https://docs.rs/topcoat/latest/topcoat/cookie/index.html): read and write the request cookie jar, with signed, encrypted, and prefixed cookies.
- [Sessions](https://docs.rs/topcoat/latest/topcoat/session/index.html): bring-your-own-storage session authentication: login/logout lifecycle, sliding expiration, and token rotation.

**Asset system**
- [Assets](https://docs.rs/topcoat/latest/topcoat/asset/index.html): declare assets in Rust, serve them with content-hashed URLs.
- [Fonts](https://docs.rs/topcoat/latest/topcoat/font/index.html): bundle and serve web fonts.
- [Icons](https://docs.rs/topcoat/latest/topcoat/icon/index.html): download Iconify icon sets or declare your own.

**Client reactivity**
- [The runtime](https://docs.rs/topcoat/latest/topcoat/runtime/index.html): signals, `$(...)` expressions, `@` event handlers, and `:` bind attributes.
- [Expressions](https://docs.rs/topcoat/latest/topcoat/runtime/macro.expr.html): the dual Rust/JavaScript expression language and its vocabulary.
- [Procedures](https://docs.rs/topcoat/latest/topcoat/runtime/attr.procedure.html): async server functions callable from the browser.
- [Shards](https://docs.rs/topcoat/latest/topcoat/runtime/attr.shard.html): components that re-render on the server when their arguments change.

**UI components**
- [Topcoat UI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/ui.md): premade components vendored into your project for you to edit.

**Third-party integrations**
- [Tailwind](https://docs.rs/topcoat/latest/topcoat/tailwind/index.html): Tailwind CSS without Node, wired into the asset pipeline.
- [htmx](https://docs.rs/topcoat/latest/topcoat/htmx/index.html): drive partial HTML swaps from the server with request/response header helpers.
- [Alpine AJAX](https://docs.rs/topcoat/latest/topcoat/alpine_ajax/index.html): drive partial HTML swaps from the server with Alpine AJAX's request-header conventions.

## Roadmap

Planned features we'd like to bring to Topcoat. Have an idea? [Open an issue](https://github.com/tokio-rs/topcoat/issues).

- [ ] `topcoat new` CLI command to bootstrap pre-configured projects
- [ ] Static export
- [ ] (More) reactivity (`topcoat-runtime`)
- [ ] More Topcoat UI components, full "blocks" e.g. sign-in form
- [ ] Emailing
- [ ] Better [Toasty](https://github.com/tokio-rs/toasty) integration (safely create/update records from forms without listing out all the fields)
- [ ] Validations
- [ ] `OpenAPI` endpoints
- [ ] Docs for how to deploy Topcoat
- [ ] Pre-rendering for static pages
- [ ] Streaming SSR / Suspense
- [ ] Client-side navigation + prefetching
- [ ] `WebSockets`
- [ ] Server-sent events
- [ ] Image optimization / resizing
- [ ] Easier-to-use middlewares like rate-limiting, compression, etc.
- [ ] Authentication
- [ ] Background jobs
- [ ] Islands
