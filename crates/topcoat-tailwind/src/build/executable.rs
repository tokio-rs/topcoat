use std::{
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::build::{BuildError, Command, Result};

const REPO: &str = "tailwindlabs/tailwindcss";

/// The Tailwind CLI release downloaded by default (without the leading `v`).
pub const DEFAULT_VERSION: &str = "4.3.2";

/// Where the Tailwind CLI executable comes from.
#[derive(Debug, Clone)]
pub enum ExecutableSource {
    /// Download the standalone CLI release from GitHub into `OUT_DIR`,
    /// reusing the copy from a previous build if present.
    Github {
        /// The release to download, without the leading `v`.
        version: String,
        /// Expected hash of the downloaded binary as an `algorithm:hex`
        /// string. Only `sha256` is currently supported, e.g.
        /// `"sha256:b800b065..."`. Verified once after download; `None` skips
        /// verification.
        checksum: Option<String>,
    },
    /// Use an existing executable. A bare command name like `"tailwindcss"`
    /// is resolved through `PATH`; anything containing a path separator is
    /// used as a file path, with relative paths resolved against the package
    /// root (the directory the build script runs in).
    Path(PathBuf),
    /// Read the executable from the named environment variable at build time,
    /// interpreting its value like [`ExecutableSource::Path`]. Print
    /// `cargo:rerun-if-env-changed=<name>` from your build script if a change
    /// to the variable should rerun it; note that printing any `rerun-if-*`
    /// directive replaces Cargo's default change detection.
    Env(String),
}

impl ExecutableSource {
    /// Resolve to a runnable [`Executable`], downloading the CLI into Cargo's
    /// `OUT_DIR` if needed. Only [`ExecutableSource::Github`] requires
    /// `OUT_DIR` to be set.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the CLI cannot be downloaded, fails checksum
    /// verification, or requires `OUT_DIR` while it is unset, or if an
    /// [`ExecutableSource::Env`] variable is unset.
    pub fn resolve(&self) -> Result<Executable> {
        match self {
            Self::Github { version, checksum } => {
                let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?);
                let dest = out_dir.join(format!("tailwindcss-{version}"));
                Self::download_from_github(version, checksum.as_deref(), dest)
            }
            Self::Path(path) => Ok(Executable::new(path)),
            Self::Env(name) => {
                let value = env::var_os(name)
                    .ok_or_else(|| BuildError::EnvNotSet { name: name.clone() })?;
                Ok(Executable::new(value))
            }
        }
    }

    /// Download the Tailwind CLI for `version` to `dest`.
    ///
    /// If `dest` already exists it's left untouched. When `checksum` is given,
    /// the downloaded file's hash is verified against it before the file is
    /// moved into place; `checksum` must carry a supported algorithm prefix
    /// (`sha256:`). On Unix the file is made executable.
    fn download_from_github(
        version: &str,
        checksum: Option<&str>,
        dest: PathBuf,
    ) -> Result<Executable> {
        // Parse the algorithm prefix up front so a malformed checksum fails
        // before anything is downloaded.
        let expected_digest = checksum
            .map(|checksum| {
                checksum
                    .strip_prefix("sha256:")
                    .ok_or_else(|| BuildError::UnsupportedChecksum {
                        checksum: checksum.to_owned(),
                    })
            })
            .transpose()?;

        if dest.exists() {
            return Ok(Executable::new(dest));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| BuildError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let url = format!(
            "https://github.com/{REPO}/releases/download/v{version}/{name}",
            name = Self::asset_name()?,
        );

        let mut body = ureq::get(&url)
            .call()
            .map_err(|e| BuildError::Http(Box::new(e)))?
            .into_body();
        let mut reader = body.as_reader();

        let temp = Self::temp_path(&dest);
        let mut file = fs::File::create(&temp).map_err(|source| BuildError::Io {
            path: temp.clone(),
            source,
        })?;
        io::copy(&mut reader, &mut file).map_err(|source| BuildError::Io {
            path: temp.clone(),
            source,
        })?;
        drop(file);

        if let Some(expected_digest) = expected_digest {
            let bytes = fs::read(&temp).map_err(|source| BuildError::Io {
                path: temp.clone(),
                source,
            })?;
            let digest = Sha256::digest(&bytes);
            let mut actual = String::with_capacity(digest.len() * 2);
            for b in &digest {
                let _ = write!(actual, "{b:02x}");
            }
            if expected_digest != actual {
                let _ = fs::remove_file(&temp);
                return Err(BuildError::ChecksumMismatch {
                    expected: format!("sha256:{expected_digest}"),
                    actual: format!("sha256:{actual}"),
                });
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp)
                .map_err(|source| BuildError::Io {
                    path: temp.clone(),
                    source,
                })?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp, perms).map_err(|source| BuildError::Io {
                path: temp.clone(),
                source,
            })?;
        }

        fs::rename(&temp, &dest).map_err(|source| BuildError::Io {
            path: dest.clone(),
            source,
        })?;

        Ok(Executable::new(dest))
    }

    /// Returns the GitHub release asset name for the host platform.
    fn asset_name() -> Result<&'static str> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        Ok(match (os, arch) {
            ("macos", "x86_64") => "tailwindcss-macos-x64",
            ("macos", "aarch64") => "tailwindcss-macos-arm64",
            ("linux", "x86_64") => "tailwindcss-linux-x64",
            ("linux", "aarch64") => "tailwindcss-linux-arm64",
            ("linux", "arm") => "tailwindcss-linux-armv7",
            ("windows", "x86_64") => "tailwindcss-windows-x64.exe",
            ("windows", "aarch64") => "tailwindcss-windows-arm64.exe",
            _ => return Err(BuildError::UnsupportedPlatform { os, arch }),
        })
    }

    /// The `.tmp` sibling of `dest` where a download is staged before the
    /// final rename.
    fn temp_path(dest: &Path) -> PathBuf {
        let file_name = dest
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .unwrap_or("tailwindcss");
        dest.with_file_name(format!("{file_name}.tmp"))
    }
}

impl Default for ExecutableSource {
    fn default() -> Self {
        Self::Github {
            version: DEFAULT_VERSION.to_owned(),
            checksum: None,
        }
    }
}

/// A Tailwind CLI executable resolved from an [`ExecutableSource`].
#[derive(Debug)]
pub struct Executable {
    path: PathBuf,
}

impl Executable {
    /// An executable at `path`. A bare command name like `"tailwindcss"` is
    /// resolved through `PATH` when run.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Run `command` with this executable and wait for it to exit.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the process cannot be spawned or exits with a
    /// non-zero status.
    pub fn run(&self, command: &Command) -> Result {
        let status = command
            .to_process(&self.path)
            .status()
            .map_err(|source| BuildError::Io {
                path: self.path.clone(),
                source,
            })?;

        if !status.success() {
            return Err(BuildError::Cli { status });
        }

        Ok(())
    }
}
