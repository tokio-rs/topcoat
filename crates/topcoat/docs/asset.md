Topcoat assets are declared from Rust code with [`asset!`](asset). The macro returns a small [`Asset`] ID and embeds the declaration into the compiled binary. After building your application, Topcoat can scan the binary, copy or download every declared file into an asset bundle directory, and serve those bundled files from the router.

# Declaring assets

Use [`asset!`](asset) anywhere in your app:

```rust
use topcoat::{
    Result,
    asset::{Asset, asset},
    router::page,
    view::view,
};

const FERRIS: Asset = asset!("./ferris.png");

#[page]
async fn about_page() -> Result {
    view! {
        <img src=(FERRIS)>
    }
}
```

You can also call the macro inline:

```rust
# use topcoat::{asset::asset, view::view};
# #[topcoat::view::component]
# async fn example() -> topcoat::Result {
view! {
    <script
        type="module"
        src=(asset!("https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.1/bundles/datastar.js"))
    ></script>
}
# }
```

When an [`Asset`] appears inside [`view!`](crate::view::view), Topcoat renders it as the URL of the bundled file. For example, an image might render as:

```html
<img src="/_topcoat/assets/ferris-1a2b3c4d5e6f7a8b.png">
```

The hash in the filename is based on the file contents, so URLs are safe to cache aggressively.

# Loading the bundle

Load the generated asset bundle while building the router, before `.build()`. Use [`AssetBundle::load`] for the default bundle location:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build()
}
```

Use [`AssetBundle::load_dir`] when you write the bundle to a custom location.

[`RouterBuilderAssetExt::assets`] does two things:

- mounts the bundle at `/_topcoat/assets`
- installs the view resolver that turns [`Asset`] values into URLs

If a page renders an [`Asset`] that is not present in the loaded bundle, rendering panics. Treat that as a build/deploy mismatch: the binary and asset bundle must come from the same build.

# Bundling

During development, `topcoat dev` builds the app and bundles assets after each successful build:

```sh
topcoat dev
# or
cargo topcoat dev
```

By default, the bundle is written to:

```text
<cargo-target>/assets
```

The download/cache directory for remote assets is:

```text
<cargo-target>/topcoat/cache/assets
```

For a manual build, use the asset subcommands:

```sh
topcoat asset list
topcoat asset bundle
topcoat asset clean
```

If Cargo would build more than one executable, choose the target:

```sh
topcoat asset bundle --bin my-app
topcoat asset bundle --package my-package
```

To write the bundle somewhere else, pass `--out` and load the same directory at runtime:

```sh
topcoat asset bundle --out dist/assets
```

```rust,no_run
# use topcoat::{asset::{AssetBundle, RouterBuilderAssetExt}, router::{Router, RouterBuilderDiscoverExt}};
let router = Router::builder()
    .discover()
    .assets(AssetBundle::load_dir("dist/assets").unwrap())
    .build();
```

When `--out` is not in one of the auto-detected locations, use [`AssetBundle::load_dir`] to point at it explicitly.

# Path resolution

The first argument to [`asset!`](asset) is a string literal path or an `http(s)` URL. Local paths are resolved by the bundler:

| Asset path | Resolution |
|---|---|
| `asset!("./ferris.png")` | relative to the source file that calls `asset!` |
| `asset!("../shared/logo.png")` | relative to the source file that calls `asset!` |
| `asset!("assets/logo.png")` | relative to the declaring crate's `CARGO_MANIFEST_DIR` |
| `asset!("/opt/app/logo.png")` | absolute path, used as-is |
| `asset!("https://example.com/logo.png")` | downloaded and cached by the bundler |

Use `./` or `../` when the asset should move with the module. Use a bare relative path when the asset is part of a crate-level assets directory.

# Output options

[`asset!`](asset) accepts optional named arguments that affect the bundled filename:

```rust
use topcoat::asset::{Asset, asset};

const RUST_LOGO: Asset = asset!(
    "https://upload.wikimedia.org/wikipedia/commons/r/rust-logo.png",
    rename: "rust",
    checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb924..."
);
```

Available options:

| Option | Meaning |
|---|---|
| `rename: "name"` | replaces the output file stem |
| `extension: "ext"` | overrides the output extension, without the leading dot |
| `checksum: "sha256:<hex>"` | requires the raw source file to match the hash (only `sha256` is supported) |

Use `checksum` for remote assets when you want deployments to fail if the remote file changes unexpectedly.

# Direct bundle access

Most Topcoat apps only need to render [`Asset`] values in [`view!`](crate::view::view). If you need the filesystem path for another purpose, load the bundle and look up the asset ID:

```rust,no_run
use topcoat::asset::{Asset, AssetBundle, asset};

const LOGO: Asset = asset!("assets/logo.png");

fn main() -> std::io::Result<()> {
    let bundle = AssetBundle::load_dir("target/assets")?;
    let logo = bundle.get(LOGO).expect("logo was bundled");
    let path = logo.path();
    Ok(())
}
```

This returns the path to the bundled file, not the original source path.
