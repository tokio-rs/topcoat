use std::{fmt, path::PathBuf};

use http::Uri;

/// Where an asset's bytes come from: a local file or a remote URL.
///
/// Returned by [`RawAsset::source`](crate::RawAsset::source).
pub enum Source {
    /// A file on the local filesystem.
    Path(PathBuf),
    /// An `http` or `https` URL that the bundler downloads.
    Url(Uri),
}

impl Source {
    /// Returns the original filename, which the bundled filename is based on.
    ///
    /// For a path this is its last component. For a URL it is the last
    /// non-empty segment of the URL path. Falls back to `"asset"` when there
    /// is no usable name.
    pub fn display_name(&self) -> String {
        match self {
            Self::Path(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("asset")
                .to_string(),
            Self::Url(uri) => uri
                .path()
                .rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or("asset")
                .to_string(),
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(p) => p.display().fmt(f),
            Self::Url(uri) => uri.fmt(f),
        }
    }
}
