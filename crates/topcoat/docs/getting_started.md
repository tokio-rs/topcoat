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

## Improving build times

As your app grows, so does the time each rebuild takes. The [build performance chapter](https://doc.rust-lang.org/cargo/guide/build-performance.html) of the Cargo book collects general advice for speeding up compilation, most of which applies directly to a Topcoat project.

## Troubleshooting

A few issues come up often when setting up a project or running the examples.

### `Address already in use`

The server binds `127.0.0.1:3000` by default, so startup fails with `Address already in use` when something else is already listening there, for example another example app that is still running. Stop the other process, or pick a different port:

```sh
PORT=8080 topcoat dev
```

### `cargo run --manifest-path` cannot find the manifest

`--manifest-path` is resolved relative to the current directory. The Topcoat example READMEs assume you run their commands from the repository root, so `cargo run --manifest-path examples/hello-world/Cargo.toml` fails with a `manifest path ... does not exist` error when run from inside an example directory. Either move to the repository root, or drop the flag and use a plain `cargo run` from the example's own directory.

### `cargo run` vs `topcoat dev`

`cargo run` builds the binary and serves the app, and that is all. `topcoat dev` additionally bundles assets before starting the server, watches your sources, and rebuilds, rebundles, and restarts on changes, with browser reload for pages that include `topcoat::dev::script()`. Use `cargo run` for a quick one-off launch of an app without assets; use `topcoat dev` for day-to-day development.

### `topcoat: command not found`

The CLI is a separate crate. Install it with `cargo install topcoat-cli`, which places a `topcoat` binary in Cargo's bin directory (usually `~/.cargo/bin`). If the shell still cannot find it, make sure that directory is in your `PATH` environment variable. The CLI is also available as `cargo topcoat ...`, which works as long as `cargo` itself is on your `PATH`.

### An example panics because assets are missing

Apps that call `AssetBundle::load()`, including several examples, expect their asset bundle to have been generated at build time. Running such an app with a plain `cargo run` panics or serves broken asset URLs if the bundle was never written. Run it under `topcoat dev`, which bundles assets automatically, or generate the bundle first with `topcoat asset bundle` and then `cargo run`.

### Repeated rustfmt warnings about unstable options

If a project's `rustfmt.toml` sets unstable options, as the Topcoat repository itself does, the stable toolchain prints a `Warning: can't set ...` line for each of them on every `cargo fmt` run and skips those options. The formatting still completes, but incompletely. To apply the full configuration and silence the warnings, install the nightly toolchain and format with it:

```sh
rustup toolchain install nightly
cargo +nightly fmt
```

## Where to next

More documentation is available in the [README](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat), which links a guide for every part of the framework.
