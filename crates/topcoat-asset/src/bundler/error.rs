use std::{io, path::PathBuf};

use http::Uri;

use crate::AssetError;

pub type BundleResult = core::result::Result<(), BundleError>;

/// Errors that can occur while bundling assets out of a binary.
#[derive(Debug)]
pub enum BundleError {
    Asset(AssetError),
    CacheIo { path: PathBuf, source: io::Error },
    Download { uri: Uri, source: reqwest::Error },
}

impl From<AssetError> for BundleError {
    fn from(error: AssetError) -> Self {
        Self::Asset(error)
    }
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asset(error) => std::fmt::Display::fmt(error, f),
            Self::CacheIo { path, source } => {
                write!(
                    f,
                    "io error for cached asset at {}: {source}",
                    path.display()
                )
            }
            Self::Download { uri, source } => {
                write!(f, "failed to download asset from {uri}: {source}")
            }
        }
    }
}

impl std::error::Error for BundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Asset(error) => Some(error),
            Self::CacheIo { source, .. } | Self::Download { source, .. } => Some(source),
        }
    }
}
