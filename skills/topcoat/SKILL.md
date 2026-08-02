---
name: topcoat
description: Build, change, review, and debug full-stack Rust web applications using Topcoat. Use for project setup, framework architecture, choosing Topcoat features, or application work spanning routing, server-rendered views, client reactivity, data, authentication, testing, assets, and integrations.
---

# Topcoat

Build through the public `topcoat` facade crate. Topcoat is early-stage, so inspect the installed version and prefer version-matched API docs over memory.

## Understand the model

- Render async pages and components on the server; let them call application services directly.
- Write HTML-like templates with `view!` and reusable functions with `#[component]`.
- Route with explicit handler paths or derive paths from the Rust module tree.
- Pass `cx: &Cx` to ordinary functions for request data, authentication, and request-scoped loading.
- Use signals and `$(...)` expressions for local browser behavior without WebAssembly.
- Use `#[shard]` for reactive server re-rendering and `#[procedure]` for imperative server calls.
- Declare content-hashed files with `asset!`; Tailwind, fonts, and icons build on the asset pipeline.

## Start a project

```sh
cargo new hello-world
cd hello-world
cargo add topcoat
cargo add tokio --features rt-multi-thread,macros
cargo install topcoat-cli
```

```rust
use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body><h1>"Hello"</h1></body>
        </html>
    }
}
```

Use `topcoat dev` for rebuilding, asset bundling, restart, and browser reload. Use `topcoat fmt` to format Topcoat macro bodies.

## Work in an existing application

1. Inspect `Cargo.toml`, router construction, `build.rs`, and nearby handlers and components.
2. Determine the routing, styling, data, session, and client-runtime patterns already in use.
3. Read the focused skill and current guide for the subsystem being changed.
4. Keep application imports on `topcoat`; its lower-level crates are implementation details.
5. Make the smallest coherent change, format it, and run focused tests followed by project checks.

In a Topcoat checkout, canonical guides live under `crates/topcoat/docs/` and each implementation crate's `docs/` directory. Runnable examples live under `examples/`.

## Keep boundaries explicit

- Prefer composable `cx` functions over hidden middleware for application concerns.
- Treat route, procedure, and shard arguments as untrusted HTTP input.
- Authorize inside every shard and procedure; page and layout checks do not guard their separate endpoints.
- Register discovered handlers with `.discover()`, module-derived handlers with `module_router!()`, and runtime values such as app context and asset bundles explicitly.
- Keep production binaries and asset bundles from the same build.
