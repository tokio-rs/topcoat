use std::{io, path::PathBuf};

pub type Result<T = ()> = std::result::Result<T, BuildError>;

/// Errors that can occur while staging Iconify icon sets.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("http error fetching {url}: {source}")]
    Http {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },
    #[error("io error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid Iconify JSON from {origin}: {source}")]
    Json {
        origin: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("`OUT_DIR` is not set; `BuildConfig::stage` must be called from a build script")]
    NoOutDir,
    #[error(
        "`CARGO_MANIFEST_DIR` is not set; `BuildConfig::stage` must be called from a build script"
    )]
    NoManifestDir,
    #[error("icon set staged as `{requested}` declares the prefix `{declared}`")]
    PrefixMismatch { requested: String, declared: String },
    #[error("alias `{alias}` in icon set `{prefix}` leads to no icon: `{parent}` is unknown")]
    UnknownAliasParent {
        prefix: String,
        alias: String,
        parent: String,
    },
    #[error("alias `{alias}` in icon set `{prefix}` is part of an alias cycle")]
    AliasCycle { prefix: String, alias: String },
}
