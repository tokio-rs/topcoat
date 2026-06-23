use std::{io, path::PathBuf};

use crate::RawAsset;

pub type Result = core::result::Result<(), AssetError>;

/// Errors that can occur while reading assets or manifests.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("io error for asset at {}: {source}", asset.source())]
    AssetIo {
        asset: RawAsset,
        #[source]
        source: io::Error,
    },
    #[error("io error for manifest at {}: {source}", path.display())]
    ManifestIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "hash mismatch for asset at {}: expected {expected}, got {actual}",
        asset.source()
    )]
    ChecksumMismatch {
        asset: RawAsset,
        expected: String,
        actual: String,
    },
}
