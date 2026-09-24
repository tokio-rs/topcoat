[Tailwind CSS](https://tailwindcss.com) generates CSS from the utility classes in your markup, such as `flex` and `text-center`.

Topcoat runs the standalone Tailwind CLI from a Cargo build script. The generated stylesheet is bundled as a Topcoat [asset](crate::asset).

# Setup

Enable the `tailwind` feature for both your runtime dependency and your build dependency:

```toml
[dependencies]
topcoat = { version = "0.9.0", features = ["tailwind"] }

[build-dependencies]
topcoat = { version = "0.9.0", default-features = false, features = ["tailwind"] }
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

[`stylesheet!`](stylesheet) returns the generated CSS as an asset. `topcoat dev` bundles it after each successful build. For a manual build, run:

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

# Build flow

Call [`BuildConfig::render`] from `build.rs`. By default, it downloads a pinned Tailwind CLI release, scans the package for classes, and writes minified CSS to `$OUT_DIR/tailwind.css`. Downloads are cached across workspace builds.

The default input is:

```css
@import "tailwindcss";
```

Cargo provides the environment variables used by the defaults. To run outside a build script, configure the executable, input, output, and working directory explicitly.

# CLI executable

By default, Topcoat downloads the Tailwind CLI from GitHub. `BuildConfig` offers alternatives:

- `version("4.3.2")`: pin the release to download.
- `version_checksum("4.3.2", "sha256:b800b065...")`: additionally verify the downloaded binary's hash. The prefix selects the algorithm; only `sha256` is supported.
- `executable("tailwindcss")`: use a preinstalled CLI instead of downloading. A bare name is resolved through `PATH`; relative paths resolve against the package root.
- `executable_env("TAILWIND_CLI")`: like `executable`, with the value read from an environment variable at build time.

A configured executable is used without downloading a copy.

# Class scanning

Tailwind scans files from the package root by default. It finds literal class names in Rust source, including `view!` markup. It cannot detect class names assembled at runtime.

Exclude build output with `.gitignore` so Tailwind does not scan generated files or stale class names. If you cannot rely on an ignore file, narrow the scan with `.cwd("src")`.

For precise control, use a custom input CSS file with Tailwind source directives. For example, an input file at the package root can scan only Rust files:

```css
@import "tailwindcss" source(none);

@source "./src/**/*.rs";
```

# Custom input CSS

The generated input is enough for default Tailwind output. Use `input(...)` when you need custom CSS, theme values, plugins supported by the standalone CLI, or Tailwind source directives:

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

[`BuildConfig::render`] prints no Cargo `rerun-if-*` directives. Cargo's default change detection reruns the build script when package files change, including when files are added or removed. Ignored files and `target/` are excluded.

If your build script prints any `rerun-if-*` directive, Cargo tracks only the paths and variables you list. Include every source that can affect the stylesheet.

Do not point `rerun-if-changed` at a directory containing `target/`. Cargo scans explicitly watched directories recursively without applying `.gitignore`, so the build output could trigger another build.

Two situations require directives of your own:

- `executable_env(name)`: print `cargo:rerun-if-env-changed=<name>` if changing the variable should rerun the build script.
- A `cwd` or `input` outside the package: Cargo's default only tracks package files, so print `cargo:rerun-if-changed` for the external paths.
