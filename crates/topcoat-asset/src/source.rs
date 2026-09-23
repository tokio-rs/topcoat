use std::{fmt, path::PathBuf};

use http::Uri;

/// Where an asset's bytes come from: a local file or a remote URL.
pub enum Source {
    Path(PathBuf),
    Url(Uri),
}

impl Source {
    /// Original filename used to derive a bundled output name (stem + ext).
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
