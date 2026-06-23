use std::{io, path::PathBuf};

use http::Uri;

use crate::AssetError;

pub type BundleResult = core::result::Result<(), BundleError>;

/// Errors that can occur while bundling assets out of a binary.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error(transparent)]
    Asset(#[from] AssetError),
    #[error("io error for cached asset at {}: {source}", path.display())]
    CacheIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to download asset from {uri}: {source}")]
    Download {
        uri: Uri,
        #[source]
        source: reqwest::Error,
    },
}
