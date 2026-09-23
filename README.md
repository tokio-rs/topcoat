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

Topcoat is a Rust framework for building web apps. Render HTML on the server, compose pages from async components, and add browser interactivity from Rust.

**Early-stage and experimental. Expect breaking changes.**

```rust,no_run
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

### HTML templates with Rust

Write HTML with `view!` and use Rust expressions and control flow inside it. Async components can load the data they need before rendering. Use `topcoat fmt` to format the templates alongside your Rust code.

### Interactivity from Rust

The runtime checks expressions as Rust and translates them to JavaScript. Signals hold state in the browser, and views update when that state changes. When an update needs server data, a shard can render fresh HTML on the server. See the [runtime guide](https://docs.rs/topcoat/latest/topcoat/runtime/index.html).

### Routing that fits your app

Register routes explicitly or discover annotated handlers. You can also derive paths from your module tree. Layouts wrap pages with shared content. See the [routing guide](https://docs.rs/topcoat/latest/topcoat/router/index.html).

### Components you can edit

[Topcoat UI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/ui.md) copies component source into your project. You can change its design and behavior directly.

### Assets declared in Rust

Declare a file with `asset!` and use its handle in a view. Topcoat bundles the file and gives it a URL based on its contents, so browsers can cache it until it changes. See the [asset guide](https://docs.rs/topcoat/latest/topcoat/asset/index.html).

## Learn Topcoat

Start with [Getting started](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md) to create an app and run the dev server.

The [API documentation](https://docs.rs/topcoat/latest/topcoat/) includes guides in each module. For working applications, browse the [examples](https://github.com/tokio-rs/topcoat/tree/main/examples) and [demos](https://github.com/tokio-rs/topcoat/tree/main/demos).

To help with the project, read the [contributing guide](https://github.com/tokio-rs/topcoat/blob/main/CONTRIBUTING.md). Questions and feature discussions are welcome in the [Tokio Discord](https://discord.gg/tokio).

## Roadmap

Follow [project issues](https://github.com/tokio-rs/topcoat/issues) for proposed features and ongoing work.
