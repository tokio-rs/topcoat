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

A few things are happening here:

- `topcoat::start` binds to `HOST` and `PORT` from the environment, defaulting to `127.0.0.1:3000`.
- `Router::builder().discover().build()` collects every `#[page]`, `#[layout]`, `#[route]`, and `#[layer]` in the binary, then finalizes the router: `home` is registered automatically.
- `topcoat::dev::script()` injects the dev-server live-reload script in debug builds and renders to nothing in release builds.
- The `hello` component is invoked from `view!` with function-call syntax.

`cargo run` is enough to serve the app, but the Topcoat CLI is what you'll want for day-to-day development.

## Install the CLI

`topcoat-cli` ships the `topcoat` binary used for the dev server, source formatting, and asset bundling. Install it from crates.io:

```sh
cargo install topcoat-cli
```

This installs a single `topcoat` executable. It is also available as a Cargo subcommand (`cargo topcoat ...`) if you prefer.

## Start the dev server

From the project root:

```sh
topcoat dev
```

This command builds the app, bundles assets, and starts the server. It watches your source directories and rebuilds, rebundles, and restarts the app on changes. Pages that include `topcoat::dev::script()` reload automatically once the new build is ready.

Open <http://127.0.0.1:3000> and you should see **Hello, World!**.

To override the bind address, set `HOST` and `PORT` before running:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

## Where to next

- [The `view!` macro](../../topcoat-view/macro/docs/view.md): templating syntax and control flow.
- [Router](router.md) and [Module-based routing](../../topcoat-router/docs/module_router.md): how pages, layouts, and API routes are wired up.
- [Request context (`Cx`)](context.md): the value pages and components read from.
- [Assets](assets.md): declare static files in Rust and serve them with content-hashed URLs.
