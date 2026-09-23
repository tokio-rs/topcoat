use std::{io, path::PathBuf};

use http::Uri;

use crate::AssetError;

/// The result of [`Bundler::bundle`](super::Bundler::bundle).
pub type BundleResult = core::result::Result<(), BundleError>;

/// An error from [`Bundler::bundle`](super::Bundler::bundle).
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// Reading an asset or writing the bundle failed.
    #[error(transparent)]
    Asset(#[from] AssetError),
    /// Reading or writing the download cache failed.
    #[error("io error for cached asset at {}: {source}", path.display())]
    CacheIo {
        /// The path in the cache that could not be read or written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Downloading a remote asset failed.
    #[error("failed to download asset from {uri}: {source}")]
    Download {
        /// The URL of the asset.
        uri: Uri,
        /// The underlying HTTP error.
        #[source]
        source: Box<ureq::Error>,
    },
    /// Two assets with the same bundled filename have different content
    /// types.
    ///
    /// Give one of them a different `rename` so that they get separate files.
    #[error(
        "conflicting content types {first:?} and {second:?} for bundled file {file}: \
         serving one file as two content types needs a different `rename` on one of \
         the declarations"
    )]
    ConflictingContentTypes {
        /// The bundled filename.
        file: String,
        /// The content type of the first declaration.
        first: String,
        /// The content type of the conflicting declaration.
        second: String,
    },
}
