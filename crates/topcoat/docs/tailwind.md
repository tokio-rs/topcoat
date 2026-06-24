Topcoat's Tailwind integration is a thin Rust wrapper around the standalone Tailwind CSS CLI. It does not run Node, `PostCSS`, or a Vite-style asset pipeline. Instead, a Cargo build script runs Tailwind, writes a CSS file into `OUT_DIR`, and the normal Topcoat asset bundler serves that CSS file with a content-hashed URL.

# Setup

Enable the `tailwind` feature for both your runtime dependency and your build dependency:

```toml
[dependencies]
topcoat = { version = "0.1", features = ["tailwind"] }

[build-dependencies]
topcoat = { version = "0.1", default-features = false, features = ["tailwind"] }
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
    view::view,
};

#[layout]
async fn layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>
                (slot.await?)
            </body>
        </html>
    }
}
```

`tailwind::stylesheet!()` expands to:

```rust,ignore
topcoat::asset::asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))
```

That means the generated CSS is just a Topcoat asset. During development, `topcoat dev` builds the app, runs the build script, bundles assets, and then serves the bundled CSS from `/_topcoat/assets/...`. For manual builds, bundle assets the same way as any other Topcoat app:

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
    .assets(AssetBundle::load_dir("target/assets").unwrap())
    .build();
```

# Build flow

`BuildConfig::render()` is intended to run from `build.rs`. It requires Cargo's `OUT_DIR` and `CARGO_MANIFEST_DIR` environment variables.

The default build does this:

1. Downloads the standalone Tailwind CLI release into `OUT_DIR` if it is not already present.
2. Generates an input CSS file in `OUT_DIR` containing:

   ```css
   @import "tailwindcss";
   ```

3. Runs Tailwind with:

   ```sh
   tailwindcss -i <input> -o <output> --cwd <cwd> --minify
   ```

4. Writes the output to `$OUT_DIR/tailwind.css`.

The default Tailwind CLI version is pinned by Topcoat to `4.3.0`. The downloaded binary is cached inside Cargo's build output directory as `tailwindcss-<version>`.

# Class scanning

Topcoat does not inspect `view!` macros or extract class names itself. Class detection is delegated to the Tailwind CLI.

By default, Topcoat passes:

```text
--cwd $CARGO_MANIFEST_DIR/src
```

So Tailwind scans from your crate's `src` directory. This works with classes in Rust source files, including literal `class="..."` values in `view!` markup. Classes assembled dynamically at runtime are still invisible to Tailwind unless you include them through your Tailwind input/configuration.

If your templates, components, or shared UI live somewhere else, change the working directory:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .cwd(".")
        .render()
        .unwrap();
}
```

For more precise control, use a custom input CSS file and Tailwind's own source configuration features from that file.

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

The input file is registered with Cargo as `rerun-if-changed`, so changing it reruns the build script.

# Configuration

`BuildConfig` exposes the options that Topcoat passes to the CLI:

| Method | Default | Meaning |
|---|---:|---|
| `version("4.3.0")` | `4.3.0` | Tailwind CLI release to download, without the leading `v`. |
| `input(path)` | generated in `OUT_DIR` | CSS input passed with `-i`. |
| `output(path)` | `$OUT_DIR/tailwind.css` | CSS output passed with `-o`. |
| `cwd(path)` | `$CARGO_MANIFEST_DIR/src` | Working directory passed with `--cwd`. |
| `optimize(bool)` | `false` | Adds `--optimize` when true. |
| `minify(bool)` | `true` | Adds `--minify` when true. |

For example:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .version("4.3.0")
        .input("src/styles/app.css")
        .cwd(".")
        .optimize(true)
        .minify(true)
        .render()
        .unwrap();
}
```

# Custom output paths

The convenience macro `tailwind::stylesheet!()` assumes the default output path:

```text
$OUT_DIR/tailwind.css
```

If you change `output(...)`, link the same file with `asset!` instead of `tailwind::stylesheet!()`:

```rust,ignore
use topcoat::asset::asset;

view! {
    <link
        rel="stylesheet"
        href=(asset!(concat!(env!("OUT_DIR"), "/app.css")))
    >
}
```

Keep the build script and the linked asset path in sync:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    topcoat::tailwind::BuildConfig::new()
        .output(out_dir.join("app.css"))
        .render()
        .unwrap();
}
```

# Rebuild behavior

`BuildConfig::render()` prints Cargo directives for:

- the input CSS file
- the configured Tailwind working directory

With the default `cwd`, any change under `src` reruns the build script and regenerates Tailwind output. `topcoat dev` also watches source directories, rebuilds the Rust binary, rebundles assets, and restarts the app after a successful build.

# Supported platforms

Topcoat downloads the Tailwind CLI asset that matches the host platform. The currently supported targets are:

| OS | Architecture |
|---|---|
| macOS | `x86_64`, `aarch64` |
| Linux | `x86_64`, `aarch64`, `arm` |
| Windows | `x86_64`, `aarch64` |

Unsupported platforms fail during the build script before Tailwind runs.
