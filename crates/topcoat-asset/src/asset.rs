use std::path::{Component, Path, PathBuf};

use http::Uri;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use topcoat_core::fnv1a::Fnv1a;

use crate::{AssetOptions, ConstReader, ConstWriter, Source};

/// A handle to an asset declared with [`asset!`](crate::asset).
///
/// An `Asset` is cheap to copy. It points at the declaration that
/// [`asset!`](crate::asset) embeds into the compiled binary. When you use it
/// in a view, it renders as the URL of its bundled file. Call
/// [`id`](Self::id) to get the [`AssetId`] the bundle stores it under.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asset(&'static [u8]);

impl Asset {
    #[doc(hidden)]
    #[must_use]
    pub const fn new(inner: &'static [u8]) -> Self {
        Self(inner)
    }

    /// Returns the [`AssetId`] that identifies this asset.
    ///
    /// # Panics
    ///
    /// Panics if the handle does not point at a valid declaration. Handles
    /// returned by [`asset!`](crate::asset) are always valid.
    #[must_use]
    #[track_caller]
    pub fn id(&self) -> AssetId {
        // The bundler discovers assets by scanning the binary for these
        // bytes; reading them through black_box stops the optimizer from
        // folding the load and stripping them out.
        let bytes = core::hint::black_box(self.0);
        let mut reader = ConstReader::new(bytes);
        reader.skip(SCRAMBLED_PREFIX.len());
        AssetId(
            reader
                .read_u64_le()
                .expect("raw asset does not have a valid ID"),
        )
    }
}

impl std::fmt::Debug for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Asset))
            .field("id", &self.id())
            .finish()
    }
}

/// A compact identifier for an asset declared with [`asset!`](crate::asset).
///
/// The ID is a hash of the declaring crate name, the source file path, the
/// asset path, and the [`AssetOptions`]. It stays the same across builds as
/// long as none of these change. Bundles use it as the key for their files:
/// get the ID of a handle with [`Asset::id`], and look up its bundled file
/// with [`AssetBundle::get`](crate::AssetBundle::get).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(u64);

impl AssetId {
    /// Builds an asset ID from the same inputs [`asset!`](crate::asset) uses.
    ///
    /// Most code should declare assets with [`asset!`](crate::asset) and read
    /// the ID with [`Asset::id`] instead. This constructor is for tooling and
    /// tests that need to compute an ID from its parts.
    #[must_use]
    pub const fn new(
        crate_name: &str,
        source_file: &str,
        path: &str,
        options: &AssetOptions,
    ) -> Self {
        let h = Fnv1a::<u64>::new()
            .write(crate_name.as_bytes())
            .write(b"\0")
            .write(source_file.as_bytes())
            .write(b"\0")
            .write(path.as_bytes());
        Self(options.hash_into(h).finish())
    }

    /// Returns the raw `u64` value of this ID.
    ///
    /// This is useful to mix an asset into another hash at compile time.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// An asset declaration read back from a compiled binary.
///
/// It holds the [`AssetId`], the declared path, the [`AssetOptions`], and the
/// crate and source file of the declaration, which are needed to resolve a
/// relative path to a file on disk. [`RawAsset::find_in_binary`] returns every
/// declaration in a binary.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAsset {
    id: AssetId,
    path: String,
    crate_name: String,
    manifest_dir: String,
    source_file: String,
    options: AssetOptions,
}

/// The size in bytes of one encoded asset declaration in the binary.
pub const ENCODED_ASSET_SIZE: usize = 2048;

impl RawAsset {
    /// Encodes a declaration into the bytes that [`asset!`](crate::asset)
    /// embeds into the binary.
    ///
    /// [`decode`](Self::decode) and [`find_in_binary`](Self::find_in_binary)
    /// read these bytes back.
    #[must_use]
    pub const fn encode(
        id: AssetId,
        path: &str,
        crate_name: &str,
        manifest_dir: &str,
        source_file: &str,
        options: &AssetOptions,
    ) -> [u8; ENCODED_ASSET_SIZE] {
        let mut out = [0u8; ENCODED_ASSET_SIZE];
        let mut w = ConstWriter::new(&mut out);
        w.write_bytes(&asset_prefix());
        w.write_u64_le(id.0);
        w.write_str(path);
        w.write_str(crate_name);
        w.write_str(manifest_dir);
        w.write_str(source_file);
        options.encode_into(&mut w);
        out
    }

    /// Decodes a declaration from the start of `buffer`.
    ///
    /// Returns `None` if `buffer` does not start with a valid encoded
    /// declaration.
    #[must_use]
    pub fn decode(buffer: &[u8]) -> Option<Self> {
        let mut r = ConstReader::new(buffer);
        r.skip(asset_prefix().len())?;
        Some(Self {
            id: AssetId(r.read_u64_le()?),
            path: r.read_str()?.to_owned(),
            crate_name: r.read_str()?.to_owned(),
            manifest_dir: r.read_str()?.to_owned(),
            source_file: r.read_str()?.to_owned(),
            options: AssetOptions::decode_from(&mut r)?,
        })
    }

    /// Returns the options the asset was declared with.
    #[must_use]
    pub fn options(&self) -> &AssetOptions {
        &self.options
    }

    /// Finds every asset declaration embedded in a compiled binary.
    ///
    /// The declarations are returned in the order they appear in the binary.
    #[must_use]
    pub fn find_in_binary(binary: &[u8]) -> Vec<Self> {
        let prefix = asset_prefix();
        let finder = memmem::Finder::new(&prefix);
        finder
            .find_iter(binary)
            .filter_map(|index| Self::decode(&binary[index..]))
            .filter(|asset| !asset.path.is_empty())
            .collect()
    }

    /// Returns the ID of the asset.
    #[must_use]
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// Returns the path or URL exactly as it was passed to
    /// [`asset!`](crate::asset).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns where the asset's bytes come from.
    ///
    /// An `http` or `https` URL becomes [`Source::Url`]. Anything else is a
    /// file, resolved with [`resolved_path`](Self::resolved_path).
    #[must_use]
    pub fn source(&self) -> Source {
        if let Ok(uri) = self.path.parse::<Uri>()
            && matches!(uri.scheme_str(), Some("http" | "https"))
        {
            return Source::Url(uri);
        }
        Source::Path(self.resolved_path())
    }

    /// Returns the name of the crate that declared the asset.
    #[must_use]
    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    /// Resolves the asset path to a filesystem path.
    ///
    /// Absolute paths are used as they are. Paths starting with `./` or `../`
    /// are relative to the directory of the source file that called
    /// [`asset!`](crate::asset). Other relative paths are relative to the
    /// declaring crate's manifest directory.
    pub fn resolved_path(&self) -> PathBuf {
        let path = Path::new(&self.path);
        if path.is_absolute() {
            return normalize(path);
        }

        let anchor = match path.components().next() {
            Some(Component::CurDir | Component::ParentDir) => {
                let source = anchor_source_file(&self.manifest_dir, &self.source_file);
                source
                    .parent()
                    .map_or_else(|| PathBuf::from(&self.manifest_dir), Path::to_path_buf)
            }
            _ => PathBuf::from(&self.manifest_dir),
        };

        normalize(&anchor.join(path))
    }
}

/// Locate the source file on disk by walking up from `manifest_dir`. Cargo
/// makes `file!()` relative to its invocation directory, which can be the
/// crate or the workspace root; trying parents in order finds whichever
/// anchor produces an existing file. Absolute paths (e.g. dependencies
/// built from the cargo cache) short-circuit.
fn anchor_source_file(manifest_dir: &str, source_file: &str) -> PathBuf {
    let source = Path::new(source_file);
    if source.is_absolute() {
        return source.to_path_buf();
    }
    let mut cur = Path::new(manifest_dir);
    loop {
        let candidate = cur.join(source);
        if candidate.exists() {
            return candidate;
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => return Path::new(manifest_dir).join(source),
        }
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(comp);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Declares an asset and returns its [`Asset`] handle.
///
/// The first argument is the asset's location, either a file path or an
/// `http://` or `https://` URL. The remaining arguments set
/// [`AssetOptions`] with the same syntax as
/// [`asset_options!`](crate::asset_options).
///
/// The path and the options must be constant expressions, usually string
/// literals. They cannot be computed at runtime.
///
/// # Discovery
///
/// The macro embeds the declaration into the compiled binary, and the
/// [`Bundler`](crate::Bundler) finds it by scanning the binary. The returned
/// [`Asset`] handle is what keeps the declaration in the binary. If no code
/// uses the handle, the compiler may remove the declaration and the bundler
/// will not see it.
///
/// # Path resolution
///
/// Local paths are resolved when the [`Bundler`](crate::Bundler) runs, not
/// when the macro expands:
///
/// - Paths starting with `./` or `../` are relative to the directory of the source file that calls
///   the macro.
/// - Other relative paths are relative to the declaring crate's `CARGO_MANIFEST_DIR`.
/// - Absolute paths are used as they are.
/// - `http://` and `https://` URLs are downloaded by the bundler and cached on disk.
///
/// # Options
///
/// Each named argument sets a field of [`AssetOptions`], which documents
/// them all. All of them are optional:
///
/// - `rename: "name"`: replaces the file stem (everything before the last `.`) of the bundled file.
/// - `extension: "ext"`: replaces the extension of the bundled file, without the leading dot.
///   Useful when the source has no extension or a wrong one.
/// - `checksum: "sha256:<hex>"`: the expected hash of the source file. The bundler fails if the
///   file does not match. Recommended for remote assets.
/// - `content_type: "text/css"`: the `Content-Type` the asset is served with. Without it, the type
///   is guessed from the extension of the bundled file.
///
/// Bundled filenames always contain a short hash of the file contents, for
/// example `logo-1a2b3c4d5e6f7a8b.png`, or `1a2b3c4d5e6f7a8b.png` when the
/// stem is empty. This makes the files safe to cache forever. Declarations
/// that produce the same filename share one bundled file, so they must agree
/// on its `Content-Type`. To serve the same file with two content types, give
/// one of the declarations a different `rename`. Otherwise the
/// [`Bundler`](crate::Bundler) fails.
///
/// # Returns
///
/// An [`Asset`] handle that can be stored in a `const`. Its [`AssetId`],
/// read with [`Asset::id`], stays the same across builds as long as the
/// declaring crate, the source file, the path string, and the options stay
/// the same. Changing the file contents does not change the ID.
///
/// # Examples
///
/// ```rust
/// use topcoat_asset::{Asset, asset};
///
/// // Anchored to the crate root.
/// const LOGO: Asset = asset!("assets/logo.png");
///
/// // Anchored to this source file's directory.
/// const SHADER: Asset = asset!("./shaders/frag.wgsl");
///
/// // Remote asset with a pinned hash and a custom output name.
/// const FONT: Asset = asset!(
///     "https://example.com/font.woff2",
///     rename: "primary",
///     checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb924",
/// );
/// ```
#[macro_export]
macro_rules! asset {
    ($path:expr $(, $($ao:tt)*)?) => {{
        const PATH: &str = $path;
        const CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
        const SOURCE_FILE: &str = file!();
        const OPTIONS: $crate::AssetOptions = $crate::asset_options!($($($ao)*)?);
        const ID: $crate::AssetId = $crate::AssetId::new(CRATE_NAME, SOURCE_FILE, PATH, &OPTIONS);

        pub static ENCODED_ASSET: [u8; $crate::ENCODED_ASSET_SIZE] = $crate::RawAsset::encode(
            ID,
            PATH,
            CRATE_NAME,
            MANIFEST_DIR,
            SOURCE_FILE,
            &OPTIONS,
        );

        $crate::Asset::new(ENCODED_ASSET.as_slice())
    }};
}

const PREFIX_KEY: u8 = 0xA7;

// "TOPCOAT_ASSET" XOR'd byte-by-byte with PREFIX_KEY. Storing the scrambled
// form means the literal marker only appears in binaries that actually carry
// an asset (where `asset_prefix` unscrambles it into the embedded payload),
// not in every binary that just links this crate.
const SCRAMBLED_PREFIX: [u8; 13] = [
    b'T' ^ PREFIX_KEY,
    b'O' ^ PREFIX_KEY,
    b'P' ^ PREFIX_KEY,
    b'C' ^ PREFIX_KEY,
    b'O' ^ PREFIX_KEY,
    b'A' ^ PREFIX_KEY,
    b'T' ^ PREFIX_KEY,
    b'_' ^ PREFIX_KEY,
    b'A' ^ PREFIX_KEY,
    b'S' ^ PREFIX_KEY,
    b'S' ^ PREFIX_KEY,
    b'E' ^ PREFIX_KEY,
    b'T' ^ PREFIX_KEY,
];

const fn asset_prefix() -> [u8; 13] {
    let mut out = [0u8; 13];
    let mut i = 0;
    while i < SCRAMBLED_PREFIX.len() {
        out[i] = SCRAMBLED_PREFIX[i] ^ PREFIX_KEY;
        i += 1;
    }
    out
}
