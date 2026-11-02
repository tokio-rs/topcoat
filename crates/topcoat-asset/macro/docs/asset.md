Declares a file and returns its [`Asset`] handle.

The first argument is a string literal with the asset's location: a path or an `http://` or `https://` URL. Options follow as `name: value` pairs.

```rust
use topcoat::asset::{Asset, asset};

const LOGO: Asset = asset!("./logo.png");

const FONT: Asset = asset!(
    "https://example.com/font.woff2",
    rename: "primary",
    checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
);
```

# Paths

- Paths starting with `./` or `../` are relative to the directory of the source file containing the call.
- Paths starting with `{mod}/` are relative to the directory where the current module's child modules belong. For both `src/about.rs` and `src/about/mod.rs`, this is `src/about/`.
- Paths starting with `{out_dir}/` are relative to the build script's output directory.
- Absolute paths are used as written.
- Other paths are relative to the directory containing the crate's `Cargo.toml`.

Compilation fails if a local file does not exist. Remote files are downloaded by the bundler and cached on disk.

# Options

- `rename`: replaces the file stem in the bundled output. The extension and content hash are kept. An empty string leaves just the hash and extension.
- `extension`: replaces the output extension, written without the leading dot. An empty string drops the extension.
- `checksum`: the expected hash of the source file, written as `"sha256:<hex>"`. Bundling fails if the file's hash differs. Recommended for remote files.
- `content_type`: the `Content-Type` the file is served with. By default, it is guessed from the bundled file's extension.

Bundled filenames include a content hash, such as `logo-1a2b3c4d5e6f7a8b.png`. Declarations that produce the same filename must agree on its content type. To serve one file with different content types, give the declarations different `rename` values.

# Bundling

The bundler finds the declarations in the compiled binary. Handles that are never used may be removed during compilation, which leaves their files out of the bundle.

The asset's [`AssetId`] depends on the crate, the file's path relative to the crate's `Cargo.toml` directory (or the URL), and the options. Declaring the same file with the same options twice in one crate gives the same ID.

[`Asset`]: struct.Asset.html
[`AssetId`]: struct.AssetId.html
