[Tailwind CSS](https://tailwindcss.com) generates CSS from the utility classes in your markup.

Topcoat runs Tailwind from a Cargo build script and serves the generated stylesheet as an [asset](crate::asset).

# Setup

Enable the `tailwind` feature for both your runtime dependency and your build dependency:

```toml
[dependencies]
topcoat = { version = "0.8.1", features = ["tailwind"] }

[build-dependencies]
topcoat = { version = "0.8.1", default-features = false, features = ["tailwind"] }
```

Add a `build.rs` next to `Cargo.toml`:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new().render().unwrap();
}
```

Then link the generated stylesheet from your layout:

```rust,ignore
use topcoat::{
    Result,
    router::{Slot, layout},
    tailwind,
    view::{View, view},
};

#[layout]
async fn layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>
                (slot)
            </body>
        </html>
    })
}
```

`topcoat dev` regenerates and bundles the stylesheet during development. For a manual build, bundle assets before starting the app:

```sh
topcoat asset bundle
```

At runtime, load the asset bundle on the router:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

let router = Router::builder()
    .discover()
    .assets(AssetBundle::load().unwrap())
    .build();
```

# Build configuration

[`BuildConfig::render`] downloads and caches the standalone Tailwind CLI, scans the package, and writes minified CSS to `$OUT_DIR/tailwind.css`. Its default input contains `@import "tailwindcss";`.

The default release is [`DEFAULT_VERSION`]. Configure the executable and paths when you need to run outside a Cargo build script.

# CLI executable

By default, Topcoat downloads the Tailwind CLI from GitHub. `BuildConfig` offers alternatives:

- `version("4.3.2")`: pin the release to download.
- `version_checksum("4.3.2", "sha256:b800b065...")`: additionally verify the downloaded binary's hash. The prefix selects the algorithm; only `sha256` is supported.
- `executable("tailwindcss")`: use a preinstalled CLI instead of downloading. A bare name is resolved through `PATH`; relative paths resolve against the package root.
- `executable_env("TAILWIND_CLI")`: like `executable`, with the value read from an environment variable at build time.

A user-provided executable is used as-is: no download happens and no network access is needed, which suits offline and sandboxed builds.

# Class scanning

Tailwind scans from the package root by default. Write complete class names in your source, including in `view!` markup. Classes assembled dynamically at runtime are invisible to Tailwind.

Keep build artifacts out of the scan with `.gitignore`, or limit the scan with `.cwd("src")`.

For precise control, use a custom input CSS with Tailwind's own source directives, e.g. to scan only Rust files:

```css
@import "tailwindcss" source(none);

@source "./src/**/*.rs";
```

# Custom input CSS

Use `input(...)` to provide your own stylesheet:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("src/styles/app.css")
        .render()
        .unwrap();
}
```

Example input:

```css
@import "tailwindcss";

@theme {
  --font-sans: Inter, sans-serif;
}
```

# Rebuild behavior

`BuildConfig::render()` leaves Cargo's change tracking unchanged. By default, Cargo reruns the build script when package files change, including added and removed files.

If your build script prints any `rerun-if-*` directives, include every path and variable that should trigger a rebuild.

Never point a directory directive at a directory that contains `target/`. Cargo scans directories recursively and ignores `.gitignore` when it does, so the build script would rerun on its own output.

Two situations require directives of your own:

- `executable_env(name)`: print `cargo:rerun-if-env-changed=<name>` if changing the variable should rerun the build script.
- A `cwd` or `input` outside the package: Cargo's default only tracks package files, so print `cargo:rerun-if-changed` for the external paths.

[`BuildConfig::render`]: BuildConfig::render
[`DEFAULT_VERSION`]: DEFAULT_VERSION
