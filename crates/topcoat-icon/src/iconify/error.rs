use std::{io, path::PathBuf};

/// The result of staging Iconify icon sets.
pub type Result<T = ()> = std::result::Result<T, BuildError>;

/// An error that occurred while staging Iconify icon sets.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Downloading an icon set failed.
    #[error("http error fetching {url}: {source}")]
    Http {
        /// The URL that was requested.
        url: String,
        /// The underlying HTTP error.
        #[source]
        source: Box<ureq::Error>,
    },
    /// Reading or writing a file failed.
    #[error("io error at {}: {source}", path.display())]
    Io {
        /// The file that could not be read or written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// An icon set is not valid `IconifyJSON`.
    #[error("invalid Iconify JSON from {origin}: {source}")]
    Json {
        /// The URL or file path the icon set came from.
        origin: String,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// `OUT_DIR` is not set because the code does not run in a build script.
    #[error("`OUT_DIR` is not set; `BuildConfig::stage` must be called from a build script")]
    NoOutDir,
    /// `CARGO_MANIFEST_DIR` is not set because the code does not run in a
    /// build script.
    #[error(
        "`CARGO_MANIFEST_DIR` is not set; `BuildConfig::stage` must be called from a build script"
    )]
    NoManifestDir,
    /// An icon set declares a different prefix than the name it was staged
    /// as.
    #[error("icon set staged as `{requested}` declares the prefix `{declared}`")]
    PrefixMismatch {
        /// The name the set was staged as.
        requested: String,
        /// The prefix the set declares.
        declared: String,
    },
    /// An alias points to an icon or alias that does not exist.
    #[error("alias `{alias}` in icon set `{prefix}` leads to no icon: `{parent}` is unknown")]
    UnknownAliasParent {
        /// The prefix of the icon set.
        prefix: String,
        /// The alias with the unknown parent.
        alias: String,
        /// The parent name that does not exist.
        parent: String,
    },
    /// An alias points back to itself through other aliases.
    #[error("alias `{alias}` in icon set `{prefix}` is part of an alias cycle")]
    AliasCycle {
        /// The prefix of the icon set.
        prefix: String,
        /// An alias that is part of the cycle.
        alias: String,
    },
}
