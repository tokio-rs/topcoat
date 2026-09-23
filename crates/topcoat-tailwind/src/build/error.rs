use std::{io, path::PathBuf, process::ExitStatus};

/// A result with [`BuildError`] as the error type.
pub type Result<T = ()> = std::result::Result<T, BuildError>;

/// An error while getting or running the Tailwind CLI.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Tailwind publishes no standalone CLI for the host platform.
    #[error("unsupported platform: {os}-{arch}")]
    UnsupportedPlatform {
        /// The host operating system.
        os: &'static str,
        /// The host CPU architecture.
        arch: &'static str,
    },
    /// Downloading the Tailwind CLI failed.
    #[error("http error: {0}")]
    Http(#[from] Box<ureq::Error>),
    /// A file operation failed, or the Tailwind CLI could not be started.
    #[error("io error at {}: {source}", path.display())]
    Io {
        /// The path of the file or executable.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// `OUT_DIR` is not set, but a default setting needs it. This happens
    /// when [`BuildConfig::render`](crate::BuildConfig::render) runs outside
    /// a build script.
    #[error("`OUT_DIR` is not set; `Config::render` must be called from a build script")]
    NoOutDir,
    /// `CARGO_MANIFEST_DIR` is not set, but the default
    /// [`cwd`](crate::BuildConfig::cwd) needs it. This happens when
    /// [`BuildConfig::render`](crate::BuildConfig::render) runs outside a
    /// build script.
    #[error("`CARGO_MANIFEST_DIR` is not set; `Config::render` must be called from a build script")]
    NoManifestDir,
    /// The environment variable of an
    /// [`ExecutableSource::Env`](crate::ExecutableSource::Env) is not set.
    #[error("environment variable `{name}` is not set")]
    EnvNotSet {
        /// The name of the variable.
        name: String,
    },
    /// The downloaded Tailwind CLI does not match the expected checksum.
    #[error("checksum mismatch for downloaded tailwindcss: expected {expected}, actual {actual}")]
    ChecksumMismatch {
        /// The expected checksum.
        expected: String,
        /// The actual checksum of the download, as `sha256:<hex>`.
        actual: String,
    },
    /// The checksum does not start with `sha256:`, the only supported
    /// algorithm.
    #[error("unsupported checksum {checksum:?} for tailwindcss: expected a `sha256:` prefix")]
    UnsupportedChecksum {
        /// The given checksum.
        checksum: String,
    },
    /// The Tailwind CLI exited with a non-zero status.
    #[error("tailwindcss exited with {status}")]
    Cli {
        /// The exit status of the process.
        status: ExitStatus,
    },
}
