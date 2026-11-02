use std::path::PathBuf;

use http::Uri;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use topcoat_core::fnv1a::Fnv1a;

use crate::{AssetOptions, ConstReader, ConstWriter, Source};

/// A handle to a file declared with `asset!`.
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
    /// handles returned by `asset!` are always valid.
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

/// Compact identifier for an asset declared via `asset!`.
///
/// The ID depends on the declaring crate name, the asset's location and its options.
/// The location is the path relative to the crate's `Cargo.toml` directory, or the
/// URL for remote assets. The ID stays the same while those inputs stay the same.
/// Use [`Asset::id`] to get an asset's ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(u64);

impl AssetId {
    /// Build an asset ID from the same inputs the `asset!` macro uses.
    ///
    /// Use this when reconstructing an ID from a declaration. Application code
    /// can get the ID directly from [`Asset::id`].
    #[must_use]
    pub const fn new(crate_name: &str, location: &str, options: &AssetOptions) -> Self {
        let h = Fnv1a::<u64>::new()
            .write(crate_name.as_bytes())
            .write(b"\0")
            .write(location.as_bytes());
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
/// Contains the asset's ID, its absolute path or URL, and its options.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAsset {
    id: AssetId,
    path: String,
    options: AssetOptions,
}

impl RawAsset {
    /// The number of bytes [`encode`](Self::encode) writes for a declaration.
    #[doc(hidden)]
    #[must_use]
    pub const fn encoded_len(path: &str, options: &AssetOptions) -> usize {
        SCRAMBLED_PREFIX.len() + 8 + 2 + path.len() + options.encoded_len()
    }

    /// Encodes a declaration into `N` bytes, where `N` is its
    /// [`encoded_len`](Self::encoded_len).
    #[doc(hidden)]
    #[must_use]
    pub const fn encode<const N: usize>(
        id: AssetId,
        path: &str,
        options: &AssetOptions,
    ) -> [u8; N] {
        let mut out = [0u8; N];
        Self::encode_into(id, path, options, &mut out);
        out
    }

    /// Encodes a declaration into a new vector.
    #[doc(hidden)]
    #[must_use]
    pub fn encode_vec(id: AssetId, path: &str, options: &AssetOptions) -> Vec<u8> {
        let mut out = vec![0u8; Self::encoded_len(path, options)];
        Self::encode_into(id, path, options, &mut out);
        out
    }

    const fn encode_into(id: AssetId, path: &str, options: &AssetOptions, out: &mut [u8]) {
        let mut w = ConstWriter::new(out);
        w.write_bytes(&asset_prefix());
        w.write_u64_le(id.0);
        w.write_str(path);
        options.encode_into(&mut w);
    }

    #[must_use]
    pub fn decode(buffer: &[u8]) -> Option<Self> {
        let mut r = ConstReader::new(buffer);
        r.skip(asset_prefix().len())?;
        Some(Self {
            id: AssetId(r.read_u64_le()?),
            path: r.read_str()?.to_owned(),
            options: AssetOptions::decode_from(&mut r)?,
        })
    }

    #[must_use]
    pub fn options(&self) -> &AssetOptions {
        &self.options
    }

    /// Recover every embedded asset declaration from a compiled binary.
    ///
    /// Declarations without a path are skipped. The macro emits them when it
    /// cannot resolve the path, such as in rust-analyzer.
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

    /// The asset's absolute path or URL.
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
        Source::Path(PathBuf::from(&self.path))
    }
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
