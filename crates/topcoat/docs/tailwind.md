[Tailwind CSS](https://tailwindcss.com) is a utility-first CSS framework: instead of writing custom stylesheets, you compose small single-purpose classes (like `flex`, `pt-4`, or `text-center`) directly in your markup, and Tailwind generates only the CSS for the classes you actually use.

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

`BuildConfig::render()` is intended to run from `build.rs`. It reads Cargo's `OUT_DIR` and `CARGO_MANIFEST_DIR` environment variables where a default depends on them; a fully custom configuration (executable, input, output, and cwd) runs without either.

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

The default Tailwind CLI version is pinned by Topcoat to `4.3.2`. The downloaded binary is cached inside Cargo's build output directory as `tailwindcss-<version>`.

# CLI executable

By default, Topcoat downloads the Tailwind CLI from GitHub. `BuildConfig` offers alternatives:

- `version("4.3.2")` — pin the release to download.
- `version_checksum("4.3.2", "sha256:b800b065…")` — additionally verify the downloaded binary's hash. The prefix selects the algorithm; only `sha256` is supported.
- `executable("tailwindcss")` — use a preinstalled CLI instead of downloading. A bare name is resolved through `PATH`; relative paths resolve against the package root.
- `executable_env("TAILWIND_CLI")` — like `executable`, with the value read from an environment variable at build time.

A user-provided executable is used as-is: no download happens and no network access is needed, which suits offline and sandboxed builds.

# Class scanning

Topcoat does not inspect `view!` macros or extract class names itself. Class detection is delegated to the Tailwind CLI.

By default, Topcoat passes:

```text
--cwd $CARGO_MANIFEST_DIR
```

So Tailwind scans from your package root: classes are found in Rust source files, including literal `class="..."` values in `view!` markup. Classes assembled dynamically at runtime are invisible to Tailwind.

The scan skips files matched by `.gitignore` — that is what keeps it out of `target/`. In a checkout without an ignore file it reads build artifacts, which is slow and can resurrect classes from previous builds; scope the scan down with `.cwd("src")` in that case.

For precise control, use a custom input CSS with Tailwind's own source directives, e.g. to scan only Rust files:

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

`BuildConfig::render()` prints no Cargo `rerun-if-*` directives. Cargo therefore applies its default: the build script reruns whenever any non-ignored file in the package changes. That default respects `.gitignore`, always excludes `target/`, and notices created and deleted files, so class changes anywhere in the package — including in new files — regenerate the Tailwind output.

Printing any `rerun-if-*` directive from your build script replaces that default with exactly the paths and variables you list. Keep that in mind when combining the Tailwind build with your own directives; in particular, a directory directive is scanned recursively without respecting `.gitignore`, so never print one for a directory containing `target/`. Two situations require directives of your own:

- `executable_env(name)`: print `cargo:rerun-if-env-changed=<name>` if changing the variable should rerun the build script.
- A `cwd` or `input` outside the package: Cargo's default only tracks package files, so print `cargo:rerun-if-changed` for the external paths.
