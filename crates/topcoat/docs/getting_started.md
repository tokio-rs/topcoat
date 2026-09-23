# Getting started

This guide shows how to create a new Topcoat project, install the `topcoat` CLI, and run your app with the dev server.

## Create a new project

Start with a new Cargo binary crate:

```sh
cargo new hello-world
cd hello-world
```

Add `topcoat` and `tokio` as dependencies:

```sh
cargo add topcoat
cargo add tokio --features rt-multi-thread,macros
```

Replace `src/main.rs` with:

```rust
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
            <head>
                <title>"Hello world"</title>
                topcoat::dev::script()
            </head>
            <body>
                hello(name: "World")
            </body>
        </html>
    })
}

#[component]
async fn hello(name: &str) -> Result<impl View> {
    Ok(view! {
        <h1>"Hello, " (name) "!"</h1>
    })
}
```

The `#[page("/")]` attribute registers `home` as the page at `/`, and `.discover()` adds every such page to the router. `hello` is a component that `home` renders like an HTML element. `topcoat::dev::script()` connects the page to the dev server described below, and renders nothing when the app runs on its own.

You can serve the app with `cargo run` (on <http://127.0.0.1:3000> by default), but for day-to-day development you will want the Topcoat CLI.

## Install the CLI

The `topcoat-cli` crate provides the dev server, the source formatter, and the asset bundler. Install it from crates.io:

```sh
cargo install topcoat-cli
```

This installs the `topcoat` executable, plus a `cargo-topcoat` executable so you can also run it as a Cargo subcommand (`cargo topcoat ...`). Make sure Cargo's `bin` directory is [on your `PATH`](https://rust-lang.org/tools/install/).

## Start the dev server

From the project root, run:

```sh
topcoat dev
```

This builds the app, bundles its assets, and starts it. It then watches your source files. On every change it rebuilds the app and restarts it once the new build succeeds. If a build fails, the previous version keeps running. Pages that render `topcoat::dev::script()` update in the browser as soon as the new version is serving. Press `r` in the terminal to rebuild manually.

Open <http://127.0.0.1:3000> and you should see **Hello, World!**.

To change the address the app listens on, set `HOST` and `PORT`:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

If the port is already in use, the dev server picks the next free port and prints it in the terminal.

## Improving build times

The time each rebuild takes grows with your app. The [build performance chapter](https://doc.rust-lang.org/cargo/guide/build-performance.html) of the Cargo book has general advice for faster compilation, and most of it applies to a Topcoat project as is.

## Where to next

The [README](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat) links to a guide for every part of the framework.
