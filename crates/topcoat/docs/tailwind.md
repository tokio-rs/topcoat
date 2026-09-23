[Tailwind CSS](https://tailwindcss.com) is a utility-first CSS framework. Instead of writing your own stylesheets, you put small single-purpose classes (like `flex`, `pt-4`, or `text-center`) directly in your markup, and Tailwind generates CSS for only the classes you use.

Topcoat's Tailwind integration is a small Rust wrapper around the [standalone Tailwind CLI](https://tailwindcss.com/blog/standalone-cli). It does not need Node.js or a JavaScript toolchain. A Cargo build script runs Tailwind and writes a CSS file into `OUT_DIR`. That file is an ordinary Topcoat [asset](crate::asset), so it is bundled and served with a content-hashed URL like any other asset.

# Setup

Enable the `tailwind` feature in both your normal dependency and your build dependency:

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

Then link the generated stylesheet from your layout with [`stylesheet!`](crate::tailwind::stylesheet). This example is not compiled here, because the macro only works in a crate with a build script:

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

`tailwind::stylesheet!()` expands to:

```rust,ignore
topcoat::asset::asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))
```

So the generated CSS is a normal Topcoat asset, and the [asset guide](crate::asset) applies to it. During development, `topcoat dev` builds the app (which runs the build script), bundles the assets, and serves the CSS from `/_topcoat/assets/...`. Without the dev server, bundle the assets yourself:

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

[`BuildConfig::render`](crate::tailwind::BuildConfig::render) is meant to run in `build.rs`. Some of its defaults read Cargo's `OUT_DIR` and `CARGO_MANIFEST_DIR` environment variables. A config that sets the executable, input, output, and cwd itself does not need either.

With the default config, `render` does the following:

1. Downloads the standalone Tailwind CLI, unless a cached copy exists.
2. Writes an input CSS file, `tailwind-input.css`, to `OUT_DIR`. It contains:

   ```css
   @import "tailwindcss";
   ```

3. Runs Tailwind:

   ```sh
   tailwindcss -i <input> -o <output> --cwd <cwd> --minify
   ```

4. Tailwind writes the stylesheet to `$OUT_DIR/tailwind.css`.

Topcoat downloads Tailwind CLI version `4.3.2` by default ([`DEFAULT_VERSION`](crate::tailwind::DEFAULT_VERSION)). The download is cached in Cargo's target directory at `topcoat/cache/tailwind`, so every package in the workspace shares one copy, and later builds reuse it.

# CLI executable

By default, Topcoat downloads the Tailwind CLI from GitHub. [`BuildConfig`](crate::tailwind::BuildConfig) has methods to change this:

- `version("4.3.2")`: sets the release to download.
- `version_checksum("4.3.2", "sha256:b800b065...")`: also checks the hash of the download. The only supported algorithm is `sha256`.
- `executable("tailwindcss")`: uses an installed CLI instead of downloading one. A plain name is looked up in `PATH`. A relative path is relative to the package root.
- `executable_env("TAILWIND_CLI")`: like `executable`, but reads the path from an environment variable when the build script runs.

An installed executable is used as it is. Nothing is downloaded and no network access is needed, which is useful for offline and sandboxed builds.

# Class scanning

Topcoat does not look at `view!` macros or collect class names itself. The Tailwind CLI finds the classes.

By default, Topcoat passes:

```text
--cwd $CARGO_MANIFEST_DIR
```

So Tailwind scans your package, starting at its root. It finds classes in Rust source files, including literal `class="..."` values in `view!` markup. Tailwind cannot see class names that are built at runtime, so write each class name out in full somewhere in your source.

Tailwind skips files that `.gitignore` excludes, and that is what keeps it out of `target/`. Without a `.gitignore`, it also scans build output, which is slow and can bring back classes from earlier builds. In that case, scan a smaller directory with `.cwd("src")`.

For full control, use a custom input CSS with Tailwind's own source directives. For example, to scan only Rust files:

```css
@import "tailwindcss" source(none);

@source "./src/**/*.rs";
```

# Custom input CSS

The generated input is enough for the default Tailwind output. Use `input(...)` when you need your own CSS, theme values, plugins that the standalone CLI supports, or source directives:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("src/styles/app.css")
        .render()
        .unwrap();
}
```

An example input file:

```css
@import "tailwindcss";

@theme {
  --font-sans: Inter, sans-serif;
}
```

# Rebuild behavior

`render` prints no Cargo `rerun-if-*` directives, so Cargo uses its default: it reruns the build script whenever a file in the package changes. This default skips files that `.gitignore` excludes, always skips `target/`, and notices new and deleted files. So a class change anywhere in the package, even in a new file, regenerates the Tailwind output.

If your build script prints any `rerun-if-*` directive, Cargo replaces the default with exactly the paths and variables you list. Keep this in mind when you add your own directives next to the Tailwind build.

Do not point a directory directive at a directory that contains `target/`. Cargo scans such directories recursively and does not respect `.gitignore` there, so the build script would rerun because of its own output.

Two setups need directives of your own:

- `executable_env(name)`: print `cargo:rerun-if-env-changed=<name>` if a change to the variable should rerun the build script.
- A `cwd` or `input` outside the package: Cargo's default only watches files in the package, so print `cargo:rerun-if-changed` for the outside paths.
