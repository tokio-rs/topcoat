Assets are static files, like images, scripts, and stylesheets, that your pages link to. You declare them in Rust code with [`asset!`](asset), which returns a small [`Asset`] handle. After you build your application, the `topcoat` CLI scans the binary for these declarations and copies or downloads every declared file into an asset bundle directory. The router then serves the bundled files, and each [`Asset`] renders as the URL of its file.

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

When you use an [`Asset`] inside [`view!`](crate::view::view), it renders as the URL of the bundled file. For example, the image above might render as:

```html
<img src="/_topcoat/assets/ferris-1a2b3c4d5e6f7a8b.png">
```

The filename contains a hash of the file contents. When the contents change, the URL changes too, so browsers and proxies can cache these files forever.

The CLI finds declarations by scanning the compiled binary. The returned [`Asset`] handle is what keeps a declaration in the binary. If no code uses the handle, the compiler may remove the declaration, and the file is not bundled.

# Loading the bundle

Load the asset bundle while you build the router, before `.build()`. [`AssetBundle::load`] loads the bundle from its default location:

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

Use [`AssetBundle::load_dir`] when you write the bundle to a different location.

[`RouterBuilderAssetExt::assets`] serves every bundled file under `/_topcoat/assets`, and makes each [`Asset`] in a view render as the URL of its file, as shown above.

Rendering an [`Asset`] that is not in the loaded bundle panics. This means the binary and the bundle do not match: they must come from the same build.

# Bundling

During development, `topcoat dev` builds the app and bundles its assets after every successful build:

```sh
topcoat dev
# or
cargo topcoat dev
```

By default, the bundle is written to an `assets` directory next to the executable that was scanned:

```text
<cargo-target>/<profile>/assets
```

Remote assets are downloaded into a cache directory, and later builds reuse the cached files:

```text
<cargo-target>/topcoat/cache/assets
```

To bundle without the dev server, use the asset subcommands:

```sh
topcoat asset list
topcoat asset bundle
topcoat asset clean
```

`list` prints the declared assets, `bundle` writes the bundle, and `clean` deletes the bundles and the download cache. If Cargo would build more than one executable, choose one:

```sh
topcoat asset bundle --bin my-app
topcoat asset bundle --package my-package
```

The subcommands build the application in order to scan it, and they accept the same profile flags as `cargo build`. Each profile has its own bundle, so bundle with the profile you are going to run:

```sh
topcoat asset bundle --release
topcoat asset bundle --profile my-profile
```

The profile matters for more than the output path. An asset's ID depends on the path it was declared with. A build script writes its output into `OUT_DIR`, and that path contains the target directory, the profile, and a hash. The Tailwind stylesheet from `tailwind::stylesheet!()` is such an asset. So a bundle from a `dev` build does not match a `--release` binary, even when the files are identical. In the same way, a bundle built in one checkout does not match a binary built in another. The bundle and the binary must come from the same build.

To write the bundle somewhere else, pass `--out`, and load the same directory at runtime:

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

[`AssetBundle::load`] only looks in the `assets` directory next to the executable. Load any other `--out` directory with [`AssetBundle::load_dir`].

# Path resolution

The first argument of [`asset!`](asset) is a string literal: a file path or an `http` or `https` URL. The CLI resolves local paths when it bundles:

| Asset path | Resolution |
|---|---|
| `asset!("./ferris.png")` | relative to the source file that calls `asset!` |
| `asset!("../shared/logo.png")` | relative to the source file that calls `asset!` |
| `asset!("assets/logo.png")` | relative to the declaring crate's `CARGO_MANIFEST_DIR` |
| `asset!("/opt/app/logo.png")` | absolute path, used as it is |
| `asset!("https://example.com/logo.png")` | downloaded and cached |

Use `./` or `../` for files that live next to the module that uses them. Use a path without a prefix for files in a crate-level assets directory.

# Output options

[`asset!`](asset) accepts optional named arguments after the path:

```rust
use topcoat::asset::{Asset, asset};

const RUST_LOGO: Asset = asset!(
    "https://upload.wikimedia.org/wikipedia/commons/r/rust-logo.png",
    rename: "rust",
    checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb924..."
);
```

Each argument sets a field of [`AssetOptions`], which documents them in detail:

| Option | Meaning |
|---|---|
| `rename: "name"` | replaces the stem of the bundled filename |
| `extension: "ext"` | replaces the extension of the bundled filename, without the leading dot |
| `checksum: "sha256:<hex>"` | fails the bundling if the source file has a different hash |
| `content_type: "text/css"` | sets the `Content-Type` the file is served with, instead of guessing it from the extension |

Use `checksum` on remote assets if you want bundling to fail when the remote file changes.

# Hosting assets externally

The application does not have to serve the bundled files itself. Any static file host can serve them instead, such as a CDN, an object store, or the reverse proxy in front of the app. To use one, register the bundle with [`AssetConfig::hosted_at`]:

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

With this configuration, the router adds no asset routes, and each [`Asset`] renders as `{base_url}/{bundled-filename}`. The image from earlier becomes `https://cdn.example.com/assets/ferris-1a2b3c4d5e6f7a8b.png`.

Uploading the files is up to your deployment. Write the bundle with `topcoat asset bundle --out dist/assets`, and upload that directory every time you deploy the binary it was built from. The filenames contain a content hash, so the host can cache them forever.

To build these URLs, the application only needs to know the bundled filename of each asset ID, not the files themselves. This mapping is the [`AssetCatalog`], and it is stored in the bundle's `manifest.toml`. On targets without filesystem access, such as WebAssembly, there is no bundle directory to load at runtime. Instead, embed the manifest into the binary with `include_str!`, parse it with [`Manifest::parse`], and pass it in place of the bundle:

```rust
# use topcoat::{asset::{AssetConfig, Manifest, RouterBuilderAssetExt}, router::Router};
# const MANIFEST: &str = "version = 1\nassets = []";
// `MANIFEST` is `include_str!("../dist/assets/manifest.toml")`.
let manifest = Manifest::parse(MANIFEST).unwrap();
let router = Router::builder()
    .assets(AssetConfig::hosted_at("https://static.example.com/assets", manifest))
    .build();
```
