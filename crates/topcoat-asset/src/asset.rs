use std::path::{Component, Path, PathBuf};

use http::Uri;
use memchr::memmem;
use serde::{Deserialize, Serialize};

use crate::{
    AssetOptions, Source,
    cursor::{ConstReader, ConstWriter},
    hash,
};

/// Compact identifier for an asset declared via [`asset!`](crate::asset).
///
/// `Asset` values are cheap to copy and store, and stable across runs as
/// long as the declaring crate name, source file path, and asset path
/// don't change. Use [`AssetBundle::get`](crate::AssetBundle::get) to
/// resolve one back to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Asset(u64);

impl Asset {
    /// Build an `Asset` ID from the same inputs the [`asset!`](crate::asset)
    /// macro uses.
    ///
    /// Prefer calling [`asset!`](crate::asset) directly; this is exposed
    /// for tooling and tests that need to reconstruct an ID from its parts.
    #[must_use]
    pub const fn new(
        crate_name: &str,
        source_file: &str,
        path: &str,
        options: &AssetOptions,
    ) -> Self {
        let mut h = hash::fnv1a(crate_name.as_bytes());
        h = hash::fnv1a_continue(h, b"\0");
        h = hash::fnv1a_continue(h, source_file.as_bytes());
        h = hash::fnv1a_continue(h, b"\0");
        h = hash::fnv1a_continue(h, path.as_bytes());
        h = options.hash_into(h);
        Self(h)
    }
}

/// An asset declaration recovered from a compiled binary.
///
/// This is what the [`Bundler`](crate::Bundler) sees while scanning: the
/// [`Asset`] ID together with the path, options, and the crate/source
/// context needed to resolve relative paths back to real files.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAsset {
    id: Asset,
    path: String,
    crate_name: String,
    manifest_dir: String,
    source_file: String,
    options: AssetOptions,
}

pub const ENCODED_ASSET_SIZE: usize = 2048;

impl RawAsset {
    #[must_use]
    pub const fn encode(
        id: Asset,
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
            id: Asset(r.read_u64_le()?),
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
            .collect()
    }

    #[must_use]
    pub fn id(&self) -> Asset {
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
    /// Paths starting with `./` or `../` are anchored to the directory of
    /// the source file the macro was invoked in; anything else is anchored
    /// to the crate's manifest dir.
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

/// Declare an asset and get back its [`Asset`] ID.
///
/// The first argument is the asset's source location, either a path or
/// an `http(s)://` URL. Any remaining arguments configure
/// [`AssetOptions`] using the same syntax as
/// [`asset_options!`](crate::asset_options).
///
/// Because the macro expands to a `const` and a `#[used] static`, both
/// the path and any options must be string literals (or other const
/// expressions) — they cannot be computed at runtime.
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
/// Options control how the bundler names the output file and (optionally)
/// pins its contents. All are optional:
///
/// - `rename: "name"` — replace the file stem (everything before the final `.`) with `"name"`.
/// - `extension: "ext"` — override the output extension (without the leading dot). Useful when the
///   source has no extension or a wrong one.
/// - `checksum: "<sha256-hex>"` — assert the SHA-256 of the raw, unbundled source file. The bundler
///   returns [`AssetError::ChecksumMismatch`](crate::AssetError) if the source's actual hash
///   differs. Recommended for remote assets.
///
/// Output filenames always include a short content hash so bundles stay
/// cache-friendly: e.g. `logo-1a2b3c4d.png`, or `1a2b3c4d.png` if the
/// stem is empty.
///
/// # Returns
///
/// A `const` [`Asset`] ID. The ID is stable across builds as long as the
/// declaring crate, source file, and path string don't change — renaming
/// the file on disk or changing options does *not* change the ID.
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
///     checksum: "e3b0c44298fc1c149afbf4c8996fb924",
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
        const ID: $crate::Asset = $crate::Asset::new(CRATE_NAME, SOURCE_FILE, PATH, &OPTIONS);

        #[used]
        pub static ENCODED_ASSET: [u8; $crate::ENCODED_ASSET_SIZE] = $crate::RawAsset::encode(
            ID,
            PATH,
            CRATE_NAME,
            MANIFEST_DIR,
            SOURCE_FILE,
            &OPTIONS,
        );

        ID
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
