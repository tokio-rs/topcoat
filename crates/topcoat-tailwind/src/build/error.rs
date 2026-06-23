use std::{io, path::PathBuf, process::ExitStatus};

pub type Result<T = ()> = std::result::Result<T, BuildError>;

#[derive(Debug)]
pub enum BuildError {
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },
    Http(Box<ureq::Error>),
    Io {
        path: PathBuf,
        source: io::Error,
    },
    NoOutDir,
    NoManifestDir,
    Cli {
        status: ExitStatus,
    },
}

impl From<Box<ureq::Error>> for BuildError {
    fn from(error: Box<ureq::Error>) -> Self {
        Self::Http(error)
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform { os, arch } => {
                write!(f, "unsupported platform: {os}-{arch}")
            }
            Self::Http(error) => write!(f, "http error: {error}"),
            Self::Io { path, source } => {
                write!(f, "io error at {}: {source}", path.display())
            }
            Self::NoOutDir => {
                write!(
                    f,
                    "`OUT_DIR` is not set; `Config::render` must be called from a build script"
                )
            }
            Self::NoManifestDir => {
                write!(
                    f,
                    "`CARGO_MANIFEST_DIR` is not set; `Config::render` must be called from a build script"
                )
            }
            Self::Cli { status } => write!(f, "tailwindcss exited with {status}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
