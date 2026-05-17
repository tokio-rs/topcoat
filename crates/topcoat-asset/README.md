# topcoat-asset

Declare assets in source, embed references in the compiled binary, and bundle the actual files out at build time.

The `asset!` macro hands you a compact `Asset` ID to use at runtime while invisibly embedding the metadata (path, options, source location) into your binary. After building, a `Bundler` scans the binary, copies or downloads every referenced file into an output directory with content-hashed names, and writes a manifest. At runtime, an `AssetBundle` loads that manifest and resolves `Asset` IDs back to bundled files.

The result: assets live next to the code that uses them, the build pipeline finds them automatically, and the runtime never touches a path that wasn't declared at compile time.

## Quick start

```toml
[dependencies]
topcoat-asset = { version = "0.1", features = ["bundler"] }
```

```rust
use topcoat_asset::{asset, Asset, AssetBundle, Bundler};

// Declare assets anywhere in your crate. The macro returns a const ID.
const LOGO: Asset = asset!("assets/logo.png");
const FONT: Asset = asset!(
    "https://example.com/font.woff2",
    rename: "primary",
    checksum: "e3b0c44298fc1c149afbf4c8996fb924…",
);

// Build step: scan the binary and emit a bundle directory.
let binary = std::fs::read("target/release/my-app")?;
Bundler::new("target/asset-cache")
    .bundle(&binary, "dist/assets")
    .await?;

// Runtime: load the bundle and resolve IDs back to files.
let bundle = AssetBundle::load_dir("dist/assets")?;
let path = bundle.get(LOGO).unwrap().path();
```

## The `asset!` macro

The first argument is the asset's source — a path or an `http(s)://` URL. Both must be string literals because the macro expands to a `const`.

### Path resolution

Local paths are resolved when the bundler runs:

- `./foo` or `../foo` — anchored to the directory of the source file the macro was invoked in
- `foo/bar` — anchored to the declaring crate's `CARGO_MANIFEST_DIR`
- `/abs/path` — used as-is
- `http://` or `https://` — downloaded by the bundler and cached on disk

### Options

Optional named arguments control the bundled output:

- `rename: "name"` — replace the file stem (everything before the final `.`)
- `extension: "ext"` — override the output extension (without the leading dot)
- `checksum: "<sha256-hex>"` — assert the SHA-256 of the raw, unbundled source file; the bundler fails with `ChecksumMismatch` otherwise. Recommended for remote assets.

Output filenames always include a short content hash, so bundles stay cache-friendly: e.g. `logo-1a2b3c4d.png`.

## Bundling

```rust
let bundler = Bundler::new("target/asset-cache");
bundler.bundle(&binary_bytes, "dist/assets").await?;
```

The bundler is incremental: re-running it against an existing output directory loads the previous manifest, skips files whose content hash hasn't changed, and removes files that are no longer referenced. Remote URLs are downloaded into the cache directory the first time they're seen and reused on subsequent runs.

Pass your own `reqwest::Client` with `Bundler::with_client` if you need custom timeouts, proxies, or auth.

## Serving

With the `tower` feature (enabled by default), `ServeAssetBundle` exposes a bundle as a `tower` service:

```rust
use topcoat_asset::{AssetBundle, ServeAssetBundle};

let bundle = AssetBundle::load_dir("dist/assets")?;
let service = ServeAssetBundle::new(&bundle);
// Mount at /assets in your axum/tower app.
```

Only filenames present in the bundle are served; any other path receives a 404 (or hits the configured fallback).

## Features

- `tower` *(default)* — `ServeAssetBundle` for serving a bundle over HTTP via `tower-http`
- `bundler` — the `Bundler` type (pulls in `tokio` and `reqwest`)
- `view` — integrates `Asset` with Topcoat's view system so an `Asset` can be rendered directly into a view as a URL or path
