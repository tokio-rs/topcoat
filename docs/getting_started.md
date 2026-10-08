# Getting started

Create and run a Topcoat app with `topcoat new`. For setup without the generator, see [Manual setup](https://github.com/tokio-rs/topcoat/blob/main/docs/manual_setup.md).

## Install the CLI

Install [Rust and Cargo](https://www.rust-lang.org/tools/install), then install the Topcoat CLI:

```sh
cargo install topcoat-cli
```

Make sure Cargo's binary directory is on your `PATH` so you can run `topcoat`. You can also invoke it as `cargo topcoat`.

## Create an app

Create an app with the recommended settings:

```sh
topcoat new hello-world --recommended
cd hello-world
```

This creates an app with module routing, Topcoat's browser runtime, Tailwind CSS, Topcoat UI, icons, fonts, and a todo example backed by Toasty and SQLite.

Omit `--recommended` to select features interactively, or use `--minimal` for a small app without a database or browser interaction library. See the [`topcoat new` reference](https://github.com/tokio-rs/topcoat/blob/main/docs/cli/new.md) for options.

## Run the app

The recommended setup includes database models. Generate and apply a migration to create their tables:

```sh
cargo run -- toasty migration generate
cargo run -- toasty migration apply
```

Run these commands again after changing a model. SQLite stores the data in `data.db`. Apps without database models, including the minimal preset, can skip this step.

Start the development server:

```sh
topcoat dev
```

Open <http://127.0.0.1:3000>. Pages and layouts are in `src/app.rs` and `src/app/`. The dev server rebuilds the app and refreshes the page when source files change. Press `r` in the terminal to rebuild manually.

To override the bind address, set `HOST` and `PORT` before running:

```sh
HOST=0.0.0.0 PORT=8080 topcoat dev
```

For ways to reduce rebuild times, see the Cargo book's [build performance chapter](https://doc.rust-lang.org/cargo/guide/build-performance.html).

See the [guide index](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat) for routing, components, and other features.
