use std::{io, path::PathBuf};

use crate::RawAsset;

/// A result with [`AssetError`] as the error type.
pub type Result = core::result::Result<(), AssetError>;

/// An error while reading an asset or writing a bundle.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// Reading or writing the file of an asset failed.
    #[error("io error for asset at {}: {source}", asset.source())]
    AssetIo {
        /// The asset whose file could not be read or written.
        asset: Box<RawAsset>,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Reading or writing the manifest or the bundle directory failed.
    #[error("io error for manifest at {}: {source}", path.display())]
    ManifestIo {
        /// The path that could not be read or written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The source file of an asset does not match its declared `checksum`.
    #[error(
        "hash mismatch for asset at {}: expected {expected}, got {actual}",
        asset.source()
    )]
    ChecksumMismatch {
        /// The asset whose checksum did not match.
        asset: Box<RawAsset>,
        /// The declared checksum.
        expected: String,
        /// The actual checksum of the file, as `sha256:<hex>`.
        actual: String,
    },
    /// The declared `checksum` of an asset does not start with `sha256:`,
    /// the only supported algorithm.
    #[error(
        "unsupported checksum {checksum:?} for asset at {}: expected a `sha256:` prefix",
        asset.source()
    )]
    UnsupportedChecksum {
        /// The asset with the unsupported checksum.
        asset: Box<RawAsset>,
        /// The declared checksum.
        checksum: String,
    },
}
