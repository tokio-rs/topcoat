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
        source: Box<ureq::Error>,
    },
    #[error(
        "conflicting content types {first:?} and {second:?} for bundled file {file}: \
         serving one file as two content types needs a different `rename` on one of \
         the declarations"
    )]
    ConflictingContentTypes {
        file: String,
        first: String,
        second: String,
    },
}
