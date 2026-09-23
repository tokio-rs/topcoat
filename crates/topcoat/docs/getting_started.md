# Getting started

Create a Topcoat app that serves a greeting, then run it with automatic reloads during development.

## Create a new project

Create a Cargo project:

```sh
cargo new hello-world
cd hello-world
```

Add `topcoat` and `tokio`:

```sh
cargo add topcoat
cargo add tokio --features rt-multi-thread,macros
```

Replace `src/main.rs` with:

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

Run `cargo run` to serve the app at <http://127.0.0.1:3000>. For automatic rebuilds and browser reloads, install the CLI below.

## Install the CLI

Install the Topcoat CLI:

```sh
cargo install topcoat-cli
```

This installs the `topcoat` command. You can also invoke it as `cargo topcoat`. If your shell cannot find it, add Cargo's binary directory to `PATH`.

## Start the dev server

From the project root:

```sh
topcoat dev
```

The dev server rebuilds and restarts the app when source files change. Include `topcoat::dev::script()` in your page, as above, to reload the browser after a successful build. Press `r` in the terminal to rebuild manually.

Open <http://127.0.0.1:3000> and you should see **Hello, World!**.

To override the bind address, set `HOST` and `PORT` before running:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

## Improving build times

For ways to reduce compilation time, see Cargo's [build performance guide](https://doc.rust-lang.org/cargo/guide/build-performance.html).

## Where to next

Read the [routing guide](https://docs.rs/topcoat/latest/topcoat/router/index.html) to add pages and layouts. The [API documentation](https://docs.rs/topcoat/latest/topcoat/) includes guides for each module.
