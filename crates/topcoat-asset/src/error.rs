use std::{io, path::PathBuf};

use crate::RawAsset;

pub type Result = core::result::Result<(), AssetError>;

/// Errors that can occur while reading assets or manifests.
#[derive(Debug)]
pub enum AssetError {
    AssetIo {
        asset: RawAsset,
        source: io::Error,
    },
    ManifestIo {
        path: PathBuf,
        source: io::Error,
    },
    ChecksumMismatch {
        asset: RawAsset,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssetIo { asset, source } => {
                write!(f, "io error for asset at {}: {source}", asset.source())
            }
            Self::ManifestIo { path, source } => {
                write!(f, "io error for manifest at {}: {source}", path.display())
            }
            Self::ChecksumMismatch {
                asset,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "hash mismatch for asset at {}: expected {expected}, got {actual}",
                    asset.source()
                )
            }
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AssetIo { source, .. } | Self::ManifestIo { source, .. } => Some(source),
            Self::ChecksumMismatch { .. } => None,
        }
    }
}
