use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::Asset;

/// Filename of the manifest within a bundle directory.
pub const MANIFEST_NAME: &str = "manifest.toml";
/// Current on-disk manifest format version.
pub const MANIFEST_VERSION: u32 = 1;

/// On-disk index of a bundle directory, mapping [`Asset`] IDs to files.
#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub assets: Vec<ManifestEntry>,
}

impl Manifest {
    /// Read and parse a manifest, rejecting unsupported versions.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidData`] if the file is not valid TOML or
    /// if its `version` field does not equal [`MANIFEST_VERSION`], and
    /// propagates any I/O error from reading `path`.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let toml_str = fs::read_to_string(path)?;
        let manifest: Manifest =
            toml::from_str(&toml_str).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if manifest.version != MANIFEST_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported manifest version {} (expected {})",
                    manifest.version, MANIFEST_VERSION
                ),
            ));
        }

        Ok(manifest)
    }

    /// Serialize the manifest to TOML and write it to `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization to TOML fails or if writing the file
    /// fails.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let toml_str = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, toml_str)
    }
}

/// One row in a [`Manifest`]: an asset ID, its bundled filename, the SHA-256
/// hex digest of the file's contents, and the `Content-Type` it is served with.
#[derive(Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: Asset,
    pub file: String,
    pub hash: String,
    pub content_type: String,
}
