# Getting started

This guide walks through creating a new Topcoat project, installing the CLI, and starting the dev server.

## Create a new project

Start with a fresh Cargo binary:

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
            <head>
                <title>"Hello world"</title>
                topcoat::dev::script()
            </head>
            <body>
                hello(name: "World")
            </body>
        </html>
    }
}

#[component]
async fn hello(name: &str) -> Result {
    view! {
        <h1>"Hello, " (name) "!"</h1>
    }
}
```

`cargo run` is enough to serve the app (by default on <http://127.0.0.1:3000>), but the Topcoat CLI is what you'll want for day-to-day development.

## Install the CLI

The Topcoat CLI crate contains a binary used for the dev server, source formatting, and asset bundling. Install it from crates.io:

```sh
cargo install topcoat-cli
```

This installs a single `topcoat` executable. It is also available as a Cargo subcommand (`cargo topcoat ...`) if you prefer. Make sure to [include it in your `PATH` environment variable](https://rust-lang.org/tools/install/).

## Start the dev server

From the project root:

```sh
topcoat dev
```

This command builds the app, bundles assets, and starts the server. It watches your source directories and rebuilds, rebundles, and restarts the app on changes. Pages that include `topcoat::dev::script()` reload automatically once the new build is ready. Press `r` in the terminal to trigger a rebuild manually.

Open <http://127.0.0.1:3000> and you should see **Hello, World!**.

To override the bind address, set `HOST` and `PORT` before running:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

More documentation is available in the [README](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat).
