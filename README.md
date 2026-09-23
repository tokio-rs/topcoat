<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/tokio-rs/topcoat/main/media/logo-dark.svg">
    <img src="https://raw.githubusercontent.com/tokio-rs/topcoat/main/media/logo-light.svg" alt="Topcoat" width="220">
  </picture>
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

Topcoat is a modular, batteries-included Rust framework for building full-stack apps. It prioritizes simplicity and productivity. See [Learn Topcoat](#learn-topcoat) to get started, or the [Roadmap](#roadmap) for what's coming next.

**Early-stage and experimental. Expect breaking changes.**

```rust,ignore
use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build()).await.unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                hello(name: "World")
            </body>
        </html>
    })
}

#[component]
async fn hello(name: &str) -> Result<impl View> {
    Ok(view! { <h1>"Hello, " (name) "!"</h1> })
}
```

## What makes Topcoat different

### Client reactivity without the boilerplate

Topcoat renders all markup on the server. Components can be async and query the database directly, so you do not need a separate API layer to feed the page. Interactivity does not need a round-trip to the server, though. A `$(...)` expression is ordinary, type-checked Rust. Topcoat evaluates it on the server for the first render and also translates it to JavaScript, so it runs again in the browser right away. There is no wasm bundle and no client build step:

```rust,ignore
#[component]
async fn faq(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);

    Ok(view! {
        // Runs entirely in the browser; no server round-trip.
        <button @click=$(|_e| open.set(!open.get()))>"What is Topcoat?"</button>
        <p :hidden=$(!open.get())>"A full-stack Rust framework."</p>
    })
}
```

When an update does need the server, like fresh search results, mark the component as a `#[shard]`. Topcoat renders it again on the server whenever one of its `$(...)` arguments changes, and swaps the new HTML into the page:

```rust,ignore
#[component]
async fn search(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, String::new);

    Ok(view! {
        <input @input=$(|e: Event| query.set(e.target.value))>

        // Updates as the user types.
        search_results(query: $(query.get()))
    })
}

#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl View> {
    Ok(view! {
        <ul>
            // Your own server-side code, like a database query:
            for product in search_products(cx, &query).await? {
                <li>(product.name)</li>
            }
        </ul>
    })
}
```

### Powerful, unsurprising HTML templates

The `view!` macro looks like HTML and uses plain Rust control flow, like `for` loops and `if` conditions, inside the markup:

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

The `topcoat fmt` CLI command formats `view!` bodies (and other Topcoat macros) across your codebase.

### Module-based routing

Topcoat can derive your routes from your app's module tree, without a build step. This is optional; you can also register each route by hand:

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

Topcoat UI is a component library built on [Tailwind](https://tailwindcss.com/) and inspired by [shadcn/ui](https://ui.shadcn.com/). The `topcoat ui` CLI command copies the component source into your project, so you can change how each component looks and works:

```rust,ignore
#[component]
async fn delete_card() -> Result<impl View> {
    Ok(view! {
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
    })
}
```

### Asset bundling

Declare a static file with the `asset!` macro. After a build, the bundler scans your binary for these declarations and copies (or downloads) every file into a local asset directory. Topcoat serves the files at content-hashed URLs, so browsers can cache them for a long time.

```rust,ignore
const FERRIS: Asset = asset!("./ferris.png");

view! { <img src=(FERRIS)> }
```

Topcoat also comes with helpers for web fonts and icons, including integrations for [Fontsource](https://fontsource.org/) (which hosts the Google Fonts) and [Iconify](https://icon-sets.iconify.design/).

### Built-in Tailwind support

Enable the `tailwind` feature to build [Tailwind](https://tailwindcss.com/) CSS without Node and serve it as an asset:

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
- [The `attributes!` macro](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html): reusable attribute collections built at runtime.
- [The `class!` macro](https://docs.rs/topcoat/latest/topcoat/view/macro.class.html): space-separated class lists from static and conditional entries.
- [The `live!` and `emit!` macros](https://docs.rs/topcoat/latest/topcoat/view/macro.live.html): stream slow parts of a page in after the rest, with the `suspense` and `error_boundary` components built on them.

**Routing**
- [Router](https://docs.rs/topcoat/latest/topcoat/router/index.html): pages, layouts, and API routes; manual and auto-discovered.
- [Module-based routing](https://docs.rs/topcoat/latest/topcoat/router/macro.module_router.html): derive the route table from your module tree.

**Working with requests**
- [Request context (`Cx`)](https://docs.rs/topcoat/latest/topcoat/context/index.html): the value pages, layouts, and components read request data from.
- [App context](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/app_context.md): share long-lived values across requests, keyed by type.
- [Memoization](https://docs.rs/topcoat/latest/topcoat/context/attr.memoize.html): `#[memoize]` to cache function results for the duration of a request.
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

**Miscellaneous**
- [Topcoat UI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/ui.md): premade components vendored into your project for you to edit.
- [Mail](https://docs.rs/topcoat/latest/topcoat/mail/index.html): declare mail with the `mail!` macro and deliver it through a pluggable transport.

**Third-party integrations**
- [Tailwind](https://docs.rs/topcoat/latest/topcoat/tailwind/index.html): Tailwind CSS without Node, wired into the asset pipeline.
- [htmx](https://docs.rs/topcoat/latest/topcoat/htmx/index.html): read htmx request headers and set its response headers.
- [Alpine AJAX](https://docs.rs/topcoat/latest/topcoat/alpine_ajax/index.html): read Alpine AJAX request headers to render partial responses.
- [Datastar](https://docs.rs/topcoat/latest/topcoat/datastar/index.html): patch elements and signals into the page over server-sent events.

## Roadmap

Features we plan to add to Topcoat. Have an idea? [Open an issue](https://github.com/tokio-rs/topcoat/issues).

- [ ] `topcoat new` CLI command to bootstrap pre-configured projects
- [ ] Static export
- [ ] (More) reactivity (`topcoat-runtime`)
- [ ] More Topcoat UI components, full "blocks" e.g. sign-in form
- [ ] Better [Toasty](https://github.com/tokio-rs/toasty) integration (safely create/update records from forms without listing out all the fields)
- [ ] Validations
- [ ] Localization support
- [ ] `OpenAPI` endpoints
- [ ] Docs for how to deploy Topcoat
- [ ] Pre-rendering for static pages
- [ ] Client-side navigation + prefetching
- [ ] `WebTransport`
- [ ] Image optimization / resizing
- [ ] Markdown support
- [ ] Easier-to-use middlewares like rate-limiting, compression, etc.
- [ ] Authentication
- [ ] Background jobs
- [ ] Islands
