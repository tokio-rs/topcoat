Use [`asset!`](asset) to refer to a static file in Rust. Topcoat bundles the file and gives it a URL containing a hash of its contents. Serve the bundle from your application or an external host.

# Declaring assets

Use [`asset!`](asset) anywhere in your app:

```rust
use topcoat::{
    Result,
    asset::{Asset, asset},
    router::page,
    view::{View, view},
};

const FERRIS: Asset = asset!("./ferris.png");

#[page]
async fn about_page() -> Result<impl View> {
    Ok(view! {
        <img src=(FERRIS)>
    })
}
```

You can also call the macro inline:

```rust
# use topcoat::{asset::asset, view::{View, view}};
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
Ok(view! {
    <script
        type="module"
        src=(asset!("https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.1/bundles/datastar.js"))
    ></script>
})
# }
```

When an [`Asset`] appears inside [`view!`](crate::view::view), Topcoat renders it as the URL of the bundled file. For example, an image might render as:

```html
<img src="/_topcoat/assets/ferris-1a2b3c4d5e6f7a8b.png">
```

The hash in the filename is based on the file contents, so URLs are safe to cache aggressively.

An unused asset declaration may be omitted from the bundle. Use its [`Asset`] handle in the application to include it.

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

[`RouterBuilderAssetExt::assets`] serves every bundled file under `/_topcoat/assets` and makes an [`Asset`] in a view render as the URL of its bundled file, as shown above.

If a page renders an [`Asset`] missing from the loaded bundle, rendering panics. Deploy the binary and asset bundle from the same build.

# Bundling

During development, `topcoat dev` builds the app and bundles assets after each successful build:

```sh
topcoat dev
# or
cargo topcoat dev
```

By default, the bundle is written to an `assets` directory next to the executable it was scanned from:

```text
<cargo-target>/<profile>/assets
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

The subcommands build the application to scan it, and they accept the same profile flags as `cargo build`. Bundle with the profile you are going to run, since each profile keeps its own bundle:

```sh
topcoat asset bundle --release
topcoat asset bundle --profile my-profile
```

Do not reuse a bundle from another profile or checkout, even when the source files are identical. Bundle the binary you will deploy.

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

[`AssetBundle::load`] only looks next to the executable, so any `--out` outside that location has to be loaded with [`AssetBundle::load_dir`].

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

The options are documented on [`AssetOptions`]:

| Option | Meaning |
|---|---|
| `rename: "name"` | replaces the output file stem |
| `extension: "ext"` | overrides the output extension, without the leading dot |
| `checksum: "sha256:<hex>"` | requires the raw source file to match the hash |
| `content_type: "text/css"` | sets the `Content-Type` the asset is served with, instead of guessing it from the extension |

Use `checksum` for remote assets when you want deployments to fail if the remote file changes unexpectedly.

# Hosting assets externally

To serve files from an external host, register its base URL with [`AssetConfig::hosted_at`]:

```rust,no_run
# use topcoat::{asset::{AssetBundle, AssetConfig, RouterBuilderAssetExt}, router::{Router, RouterBuilderDiscoverExt}};
let router = Router::builder()
    .discover()
    .assets(AssetConfig::hosted_at(
        "https://cdn.example.com/assets",
        AssetBundle::load().unwrap(),
    ))
    .build();
```

Registered this way, the router adds no asset routes, and an [`Asset`] in a view renders as the file's URL on the external host, `{base_url}/{bundled-filename}`. The image from earlier becomes `https://cdn.example.com/assets/ferris-1a2b3c4d5e6f7a8b.png`.

Write the bundle with `topcoat asset bundle --out dist/assets` and upload that directory when deploying the matching binary. The host can serve the files with long-lived, immutable caching.

With external hosting, the application only needs the bundle's `manifest.toml` to resolve URLs. If the target has no filesystem access, embed that manifest in the binary and pass it in place of the bundle:

```rust,ignore
let manifest = Manifest::parse(include_str!("../dist/assets/manifest.toml"))?;
let router = Router::builder()
    .assets(AssetConfig::hosted_at("https://static.example.com/assets", manifest))
    .build();
```
