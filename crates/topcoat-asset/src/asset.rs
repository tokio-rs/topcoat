use std::path::{Component, Path, PathBuf};

use http::Uri;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use topcoat_core::fnv1a::Fnv1a;

use crate::{AssetOptions, ConstReader, ConstWriter, Source};

/// A handle to a file declared with [`asset!`](crate::asset).
///
/// Rendering an asset in a view produces its bundled URL. Use
/// [`id`](Self::id) to look it up in an asset catalog. Handles are cheap to copy.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asset(&'static [u8]);

impl Asset {
    #[doc(hidden)]
    #[must_use]
    pub const fn new(inner: &'static [u8]) -> Self {
        Self(inner)
    }

    /// The compact [`AssetId`] identifying this asset.
    ///
    /// # Panics
    ///
    /// Panics if the handle does not reference a valid encoded declaration;
    /// handles returned by [`asset!`](crate::asset) are always valid.
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

/// Compact identifier for an asset declared via [`asset!`](crate::asset).
///
/// The ID depends on the declaring crate name, source file path, asset path,
/// and options. It stays the same while those inputs stay the same.
/// Use [`Asset::id`] to get an asset's ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(u64);

impl AssetId {
    /// Build an asset ID from the same inputs the [`asset!`](crate::asset)
    /// macro uses.
    ///
    /// Use this when reconstructing an ID from a declaration. Application code
    /// can get the ID directly from [`Asset::id`].
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

    /// The raw `u64` backing this ID.
    ///
    /// This value can be included in a hash at compile time.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// An asset declaration recovered from a compiled binary.
///
/// Contains the asset's ID, path, options, and the source location needed to
/// resolve relative paths.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAsset {
    id: AssetId,
    path: String,
    crate_name: String,
    manifest_dir: String,
    source_file: String,
    options: AssetOptions,
}

/// Size in bytes of one encoded asset declaration embedded into the binary.
pub const ENCODED_ASSET_SIZE: usize = 2048;

impl RawAsset {
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

    #[must_use]
    pub fn options(&self) -> &AssetOptions {
        &self.options
    }

    /// Recover every embedded asset declaration from a compiled binary.
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

    #[must_use]
    pub fn id(&self) -> AssetId {
        self.id
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Classify the asset as a filesystem path or an http(s) URL.
    #[must_use]
    pub fn source(&self) -> Source {
        if let Ok(uri) = self.path.parse::<Uri>()
            && matches!(uri.scheme_str(), Some("http" | "https"))
        {
            return Source::Url(uri);
        }
        Source::Path(self.resolved_path())
    }

    #[must_use]
    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    /// Resolve the asset path to an absolute filesystem path.
    ///
    /// Paths starting with `./` or `../` are relative to the source file's
    /// directory. Other relative paths start from the crate's manifest
    /// directory. Absolute paths are unchanged.
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

/// Locates a source file by checking `manifest_dir` and its ancestors.
/// Relative `file!()` paths may start at the crate or workspace root.
/// Absolute paths are returned unchanged.
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

/// Declares a file and returns its [`Asset`] handle.
///
/// The first argument is the asset's source location, either a path or
/// an `http(s)://` URL. Any remaining arguments configure
/// [`AssetOptions`] using the same syntax as
/// [`asset_options!`](crate::asset_options).
///
/// The path and options must be string literals or other const expressions.
///
/// # Discovery
///
/// The [`Bundler`](crate::Bundler) reads declarations from the compiled binary.
/// Unused handles may be removed during compilation, leaving their assets
/// out of the bundle.
///
/// # Path resolution
///
/// Local paths are resolved when the [`Bundler`](crate::Bundler) runs,
/// not at macro-expansion time:
///
/// - Paths starting with `./` or `../` are anchored to the directory of the source file the macro
///   was invoked in.
/// - Other relative paths are anchored to the declaring crate's `CARGO_MANIFEST_DIR`.
/// - Absolute paths are used as-is.
/// - Strings parseable as `http://` or `https://` URIs are downloaded by the bundler and cached on
///   disk.
///
/// # Options
///
/// Named arguments configure [`AssetOptions`]. For example, `rename: "logo"`
/// changes the output stem, and `checksum: "sha256:<hex>"` verifies the source.
///
/// Output filenames include a content hash, such as
/// `logo-1a2b3c4d5e6f7a8b.png`. Declarations that produce the same filename must
/// agree on its content type. To serve one file with different content types,
/// give the declarations different `rename` values.
///
/// # Returns
///
/// A `const`-compatible [`Asset`] handle. Its [`AssetId`] depends on the
/// declaring crate, source file, path string, and options.
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
