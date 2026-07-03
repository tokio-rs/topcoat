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

Downloading from GitHub is only the default. Where the executable comes from is configured with an `ExecutableSource`:

- `ExecutableSource::Github { version, checksum }` — download the release into `OUT_DIR`, reusing the copy from a previous build if present. When `checksum` is set, the downloaded binary's SHA-256 (lowercase hex) is verified before the binary is used.
- `ExecutableSource::Path(path)` — use an existing executable. A bare command name like `"tailwindcss"` is resolved through `PATH`; anything containing a path separator is used as a file path, with relative paths resolved against the package root.
- `ExecutableSource::Env(name)` — read the executable from the named environment variable at build time, interpreted like `ExecutableSource::Path`. The build fails if the variable is unset.

`BuildConfig` has a shorthand setter for each variant. Pin and verify a download:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .version_checksum(
            "4.3.2",
            "b800b0659dc64b9f03ede5660244d9415d777d5739ae2889280877ca37be742a",
        )
        .render()
        .unwrap();
}
```

Downloading needs network access on the first build. Offline and sandboxed builds (Nix, locked-down CI, `cargo build --offline`) should use a preinstalled CLI instead, either fixed:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .executable("tailwindcss") // resolved through `PATH`
        .render()
        .unwrap();
}
```

or chosen by the build environment, e.g. `TAILWIND_CLI=/usr/bin/tailwindcss`:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .executable_env("TAILWIND_CLI")
        .render()
        .unwrap();
}
```

With a user-provided executable no download happens and `version(...)` plays no role: you get whatever the binary is.

# Class scanning

Topcoat does not inspect `view!` macros or extract class names itself. Class detection is delegated to the Tailwind CLI.

By default, Topcoat passes:

```text
--cwd $CARGO_MANIFEST_DIR
```

So Tailwind scans from your package root: classes are found in Rust source files — including literal `class="..."` values in `view!` markup — and any other text files in the package. Classes assembled dynamically at runtime are invisible to Tailwind unless you include them through your Tailwind input/configuration.

Tailwind's automatic source detection walks every file under `--cwd` that is not matched by `.gitignore`, so the ignore file is load-bearing: Cargo's generated `.gitignore` excludes `target/`, but in a checkout without one (a tarball export, a Docker build context that omits dotfiles) the scan descends into build artifacts. That is slow, and because artifacts contain the class names of the previous build, removed classes can keep reappearing in the output. If the build environment cannot guarantee an ignore file, scope the scan down with `cwd`:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .cwd("src")
        .render()
        .unwrap();
}
```

The CLI resolves relative `input` and `output` paths against `--cwd` as well.

For precise control — for example scanning only Rust files — use a custom input CSS that disables automatic detection and registers explicit sources:

```css
@import "tailwindcss" source(none);

@source "./src/**/*.rs";
```

(`@source` globs resolve relative to the CSS file's own location.)

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

- `ExecutableSource::Env`: print `cargo:rerun-if-env-changed=<name>` if changing the variable should rerun the build script.
- A `cwd` or `input` outside the package: Cargo's default only tracks package files, so print `cargo:rerun-if-changed` for the external paths.
