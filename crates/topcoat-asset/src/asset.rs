use std::path::{Component, Path, PathBuf};

use http::Uri;
use memchr::memmem;
use serde::{Deserialize, Serialize};

use crate::{
    Source,
    cursor::{ConstReader, ConstWriter},
    hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Asset(u64);

impl Asset {
    pub const fn new(crate_name: &str, source_file: &str, path: &str) -> Self {
        let mut h = hash::fnv1a(crate_name.as_bytes());
        h = hash::fnv1a_continue(h, b"\0");
        h = hash::fnv1a_continue(h, source_file.as_bytes());
        h = hash::fnv1a_continue(h, b"\0");
        h = hash::fnv1a_continue(h, path.as_bytes());
        Self(h)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawAsset {
    id: Asset,
    path: String,
    crate_name: String,
    manifest_dir: String,
    source_file: String,
}

pub const ENCODED_ASSET_SIZE: usize = 2048;

impl RawAsset {
    pub const fn encode(
        id: Asset,
        path: &str,
        crate_name: &str,
        manifest_dir: &str,
        source_file: &str,
    ) -> [u8; ENCODED_ASSET_SIZE] {
        let mut out = [0u8; ENCODED_ASSET_SIZE];
        let mut w = ConstWriter::new(&mut out);
        w.write_bytes(&asset_prefix());
        w.write_u64_le(id.0);
        w.write_str(path);
        w.write_str(crate_name);
        w.write_str(manifest_dir);
        w.write_str(source_file);
        out
    }

    pub fn decode(buffer: &[u8]) -> Option<Self> {
        let mut r = ConstReader::new(buffer);
        r.skip(asset_prefix().len())?;
        Some(Self {
            id: Asset(r.read_u64_le()?),
            path: r.read_str()?.to_owned(),
            crate_name: r.read_str()?.to_owned(),
            manifest_dir: r.read_str()?.to_owned(),
            source_file: r.read_str()?.to_owned(),
        })
    }

    pub fn find_in_binary(binary: &[u8]) -> Vec<Self> {
        let prefix = asset_prefix();
        let finder = memmem::Finder::new(&prefix);
        finder
            .find_iter(binary)
            .filter_map(|index| Self::decode(&binary[index..]))
            .collect()
    }

    pub fn id(&self) -> Asset {
        self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// Classify the asset as a filesystem path or an http(s) URL.
    pub fn source(&self) -> Source {
        if let Ok(uri) = self.path.parse::<Uri>()
            && matches!(uri.scheme_str(), Some("http" | "https"))
        {
            return Source::Url(uri);
        }
        Source::Path(self.resolved_path())
    }

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
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from(&self.manifest_dir))
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

#[macro_export]
macro_rules! asset {
    ($path:expr) => {{
        const PATH: &str = $path;
        const CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
        const SOURCE_FILE: &str = file!();
        const ID: $crate::Asset = $crate::Asset::new(CRATE_NAME, SOURCE_FILE, PATH);

        #[used]
        pub static ENCODED_ASSET: [u8; $crate::ENCODED_ASSET_SIZE] =
            $crate::RawAsset::encode(ID, PATH, CRATE_NAME, MANIFEST_DIR, SOURCE_FILE);

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
