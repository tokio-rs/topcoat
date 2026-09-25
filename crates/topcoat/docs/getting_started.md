# Getting started

Create a Topcoat app and run it locally with automatic updates as you edit.

## Create a new project

Create a Cargo binary project:

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

Run `cargo run` to serve the app at <http://127.0.0.1:3000>. For automatic rebuilds while you work, install the Topcoat CLI.

## Install the CLI

Install the CLI from crates.io:

```sh
cargo install topcoat-cli
```

Make sure Cargo's binary directory is on your `PATH` so you can run `topcoat`. You can also invoke it as `cargo topcoat`.

## Start the dev server

From the project root:

```sh
topcoat dev
```

The dev server builds and starts the app, then rebuilds it when source files change. Pages that include `topcoat::dev::script()` update when the new build is ready. Press `r` in the terminal to rebuild manually.

Open <http://127.0.0.1:3000> and you should see **Hello, World!**.

To override the bind address, set `HOST` and `PORT` before running:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

## Improving build times

For ways to reduce rebuild times, see the Cargo book's [build performance chapter](https://doc.rust-lang.org/cargo/guide/build-performance.html).

## Where to next

Choose your next topic from the [guide index](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat).
