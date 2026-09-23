use std::{io, path::PathBuf, process::ExitStatus};

pub type Result<T = ()> = std::result::Result<T, BuildError>;

/// Errors that can occur while resolving or running the Tailwind CLI.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unsupported platform: {os}-{arch}")]
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },
    #[error("http error: {0}")]
    Http(#[from] Box<ureq::Error>),
    #[error("io error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("`OUT_DIR` is not set; `Config::render` must be called from a build script")]
    NoOutDir,
    #[error("`CARGO_MANIFEST_DIR` is not set; `Config::render` must be called from a build script")]
    NoManifestDir,
    #[error("environment variable `{name}` is not set")]
    EnvNotSet { name: String },
    #[error("checksum mismatch for downloaded tailwindcss: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("unsupported checksum {checksum:?} for tailwindcss: expected a `sha256:` prefix")]
    UnsupportedChecksum { checksum: String },
    #[error("tailwindcss exited with {status}")]
    Cli { status: ExitStatus },
}
