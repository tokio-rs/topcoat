use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::AssetId;

/// The filename of the manifest inside a bundle directory.
pub const MANIFEST_NAME: &str = "manifest.toml";
/// The manifest format version that this crate reads and writes.
pub const MANIFEST_VERSION: u32 = 1;

/// The index of a bundle directory, stored as TOML in
/// [`MANIFEST_NAME`].
///
/// The manifest lists every bundled asset with its ID and file. Convert it
/// into an [`AssetCatalog`](crate::AssetCatalog) to resolve asset URLs.
#[derive(Serialize, Deserialize)]
pub struct Manifest {
    /// The format version, always [`MANIFEST_VERSION`] for a loaded
    /// manifest.
    pub version: u32,
    /// One entry per asset declaration, in declaration order.
    pub assets: Vec<ManifestEntry>,
}

impl Manifest {
    /// Reads and parses the manifest file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::InvalidData`] if the file is
    /// not a valid manifest or its `version` is not [`MANIFEST_VERSION`].
    /// Returns any I/O error from reading `path`.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::parse(&fs::read_to_string(path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parses a manifest from its TOML source.
    ///
    /// This is useful on targets without filesystem access, such as
    /// WebAssembly, where you embed the manifest into the binary with
    /// `include_str!`.
    ///
    /// # Errors
    ///
    /// Returns an error if the source is not a valid manifest or its
    /// `version` is not [`MANIFEST_VERSION`].
    pub fn parse(toml_str: &str) -> Result<Self, ParseManifestError> {
        let manifest: Manifest = toml::from_str(toml_str)?;

        if manifest.version != MANIFEST_VERSION {
            return Err(ParseManifestError::UnsupportedVersion {
                found: manifest.version,
            });
        }

        Ok(manifest)
    }

    /// Writes the manifest as TOML to `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be serialized or the file
    /// cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let toml_str = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, toml_str)
    }
}

/// One asset in a [`Manifest`].
#[derive(Serialize, Deserialize)]
pub struct ManifestEntry {
    /// The ID of the asset.
    pub id: AssetId,
    /// The bundled filename, relative to the bundle directory.
    pub file: String,
    /// The SHA-256 hash of the file contents, as lowercase hex.
    pub hash: String,
    /// The `Content-Type` the file is served with.
    pub content_type: String,
}

/// An error from [`Manifest::parse`].
#[derive(Debug, thiserror::Error)]
pub enum ParseManifestError {
    /// The source is not valid manifest TOML.
    #[error("invalid manifest TOML: {0}")]
    Toml(#[from] toml::de::Error),
    /// The manifest has a format version other than [`MANIFEST_VERSION`].
    #[error("unsupported manifest version {found} (expected {})", MANIFEST_VERSION)]
    UnsupportedVersion {
        /// The version found in the manifest.
        found: u32,
    },
}
