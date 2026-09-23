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

Topcoat is a Rust framework for building web apps. Write pages, server logic, and browser interactions in Rust, and enable the features your app needs. Start with [Learn Topcoat](#learn-topcoat).

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

Topcoat renders HTML on the server. Components can await a database query and render the result directly. For browser interactions, use a `$(...)` expression. Topcoat type-checks it as Rust and translates it to JavaScript. The same expression can supply the initial value on the server and update it in the browser without a separate client build:

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

Use `#[shard]` for a component that needs server data. When one of its `$(...)` arguments changes, Topcoat renders it again on the server and updates its HTML in the page:

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

### HTML templates with Rust control flow

The `view!` macro combines HTML syntax with Rust expressions and control flow:

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

Run `topcoat fmt` to format the macro bodies in your source files.

### Module-based routing

Use `module_router!` to derive routes from your app's Rust modules:

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
    |   `-- id.rs       -> /posts/{post_id} (with segment!("/{post_id}"))
    `-- api/
        `-- health.rs   -> GET /api/health
```

### Premade components you can edit

Topcoat UI provides [Tailwind](https://tailwindcss.com/) components inspired by [shadcn/ui](https://ui.shadcn.com/). Run `topcoat ui add` to copy a component into your project, then edit its source to suit your app:

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

Declare a static file with `asset!` and use it in a view. Topcoat bundles the file and serves it at a URL based on its content, so browsers can cache it across requests.

```rust,ignore
const FERRIS: Asset = asset!("./ferris.png");

view! { <img src=(FERRIS)> }
```

See the [asset guide](https://docs.rs/topcoat/latest/topcoat/asset/index.html) for setup and bundling.

### Built-in Tailwind support

Enable the `tailwind` feature to integrate [Tailwind](https://tailwindcss.com/) into your project:

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
- [Streaming views](https://docs.rs/topcoat/latest/topcoat/view/macro.live.html): show placeholders while slow content loads.

**Routing**

- [Router](https://docs.rs/topcoat/latest/topcoat/router/index.html): register handlers and serve requests.
- [Module-based routing](https://docs.rs/topcoat/latest/topcoat/router/macro.module_router.html): derive the route table from your module tree.

**Working with requests**

- [Request context (`Cx`)](https://docs.rs/topcoat/latest/topcoat/context/index.html): access data for the current request.
- [App context](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/app_context.md): share long-lived values across requests, keyed by type.
- [Memoization](https://docs.rs/topcoat/latest/topcoat/context/attr.memoize.html): cache function results within a request.
- [Functions, not middlewares](https://docs.rs/topcoat/latest/topcoat/context/index.html#functions-not-middlewares): the recommended way to model auth and other request-scoped concerns.
- [Cookies](https://docs.rs/topcoat/latest/topcoat/cookie/index.html): read and write cookies.
- [Sessions](https://docs.rs/topcoat/latest/topcoat/session/index.html): manage session tokens with application-owned storage.

**Asset system**

- [Assets](https://docs.rs/topcoat/latest/topcoat/asset/index.html): declare assets in Rust, serve them with content-hashed URLs.
- [Fonts](https://docs.rs/topcoat/latest/topcoat/font/index.html): bundle and serve web fonts.
- [Icons](https://docs.rs/topcoat/latest/topcoat/icon/index.html): download Iconify icon sets or declare your own.

**Client reactivity**

- [The runtime](https://docs.rs/topcoat/latest/topcoat/runtime/index.html): add state and browser interactions to a page.
- [Expressions](https://docs.rs/topcoat/latest/topcoat/runtime/macro.expr.html): write expressions that run on the server and in the browser.
- [Procedures](https://docs.rs/topcoat/latest/topcoat/runtime/attr.procedure.html): async server functions callable from the browser.
- [Shards](https://docs.rs/topcoat/latest/topcoat/runtime/attr.shard.html): components that re-render on the server when their arguments change.

**Miscellaneous**

- [Topcoat UI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/ui.md): premade components vendored into your project for you to edit.
- [Mail](https://docs.rs/topcoat/latest/topcoat/mail/index.html): create and send email.

**Third-party integrations**

- [Tailwind](https://docs.rs/topcoat/latest/topcoat/tailwind/index.html): Tailwind CSS without Node, wired into the asset pipeline.
- [htmx](https://docs.rs/topcoat/latest/topcoat/htmx/index.html): drive partial HTML swaps from the server with request/response header helpers.
- [Alpine AJAX](https://docs.rs/topcoat/latest/topcoat/alpine_ajax/index.html): drive partial HTML swaps from the server with Alpine AJAX's request-header conventions.
- [Datastar](https://docs.rs/topcoat/latest/topcoat/datastar/index.html): patch elements and signals into the page over server-sent events.

## Contributing

See the [contribution guide](https://github.com/tokio-rs/topcoat/blob/main/CONTRIBUTING.md) before proposing a feature or submitting a change. Report bugs in the [issue tracker](https://github.com/tokio-rs/topcoat/issues).
