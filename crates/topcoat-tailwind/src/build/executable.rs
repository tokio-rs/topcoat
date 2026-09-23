use std::{
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};

use crate::build::{BuildError, Command, Result};

const REPO: &str = "tailwindlabs/tailwindcss";

/// The Tailwind CLI release downloaded by default (without the leading `v`).
pub const DEFAULT_VERSION: &str = "4.3.2";

/// Where the Tailwind CLI executable comes from.
#[derive(Debug, Clone)]
pub enum ExecutableSource {
    /// Downloads the standalone CLI release from GitHub into a shared cache.
    /// Reuses an existing cached copy.
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
    /// Resolve to a runnable [`Executable`], downloading the CLI into the
    /// shared Topcoat cache if needed. Only [`ExecutableSource::Github`]
    /// downloads, and it needs `OUT_DIR` set to locate the cache.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the CLI cannot be downloaded, fails checksum
    /// verification, or requires `OUT_DIR` while it is unset, or if an
    /// [`ExecutableSource::Env`] variable is unset.
    pub fn resolve(&self) -> Result<Executable> {
        match self {
            Self::Github { version, checksum } => {
                Self::download_from_github(version, checksum.as_deref())
            }
            Self::Path(path) => Ok(Executable::new(path)),
            Self::Env(name) => {
                let value = env::var_os(name)
                    .ok_or_else(|| BuildError::EnvNotSet { name: name.clone() })?;
                Ok(Executable::new(value))
            }
        }
    }

    /// Downloads `version` into the shared Topcoat cache unless already cached.
    ///
    /// Cache entries are specific to the version and host platform. A file
    /// lock prevents concurrent downloads from overwriting one another.
    /// When supplied, `checksum` must use the `sha256:` prefix and is checked
    /// before the downloaded file enters the cache. Cached files are reused
    /// without verification. On Unix, the downloaded file is made executable.
    fn download_from_github(version: &str, checksum: Option<&str>) -> Result<Executable> {
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

        let dir = Self::cache_dir()?;
        // The platform is baked into the file name so a cache directory shared
        // across hosts (e.g. a mounted target directory) never hands one host
        // another's binary.
        let file_name = format!(
            "tailwindcss-{version}-{platform}",
            platform = Self::platform()?
        );
        let dest = dir.join(&file_name);

        // Cached files are complete downloads. Verification only happens
        // during the download, when a checksum was supplied.
        if dest.exists() {
            return Ok(Executable::new(dest));
        }

        fs::create_dir_all(&dir).map_err(|source| BuildError::Io {
            path: dir.clone(),
            source,
        })?;

        // Lock across processes so one build downloads while the others wait.
        // Keep the lock until this function returns.
        //
        // Leave the lock file in place. Removing it could let a waiter and a
        // new process lock different inodes and download concurrently.
        let lock_path = dir.join(format!("{file_name}.lock"));
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| BuildError::Io {
                path: lock_path.clone(),
                source,
            })?;
        lock.lock().map_err(|source| BuildError::Io {
            path: lock_path.clone(),
            source,
        })?;

        // Re-check under the lock: another process may have finished the
        // download while we waited to acquire it.
        if dest.exists() {
            return Ok(Executable::new(dest));
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

        let temp = dir.join(format!("{file_name}.download"));
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

    /// The host platform suffix used in cache filenames.
    fn platform() -> Result<&'static str> {
        let asset = Self::asset_name()?;
        Ok(asset.strip_prefix("tailwindcss-").unwrap_or(asset))
    }

    /// The directory the downloaded CLI is cached in: the shared Topcoat cache
    /// (`topcoat/cache/tailwind`) under the Cargo target directory, falling
    /// back to `OUT_DIR` itself when the build runs outside Cargo's target
    /// layout.
    fn cache_dir() -> Result<PathBuf> {
        if let Some(dir) = topcoat_core::cache::cache_dir("tailwind") {
            return Ok(dir);
        }
        let out_dir = env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?;
        Ok(PathBuf::from(out_dir))
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
        // The CLI extracts native modules before loading them. Give each run
        // a private directory so concurrent processes cannot read files that
        // another process is still writing. Prefer a location that permits
        // loading executable files.
        let scratch = ScratchDir::new(&self.scratch_root())?;

        let status = command
            .to_process(&self.path)
            .env("TMPDIR", scratch.path())
            .env("TMP", scratch.path())
            .env("TEMP", scratch.path())
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

    /// Selects a parent for each run's temporary directory. Prefers `OUT_DIR`,
    /// then the executable's directory, then the system temporary directory.
    fn scratch_root(&self) -> PathBuf {
        if let Some(out_dir) = env::var_os("OUT_DIR") {
            return PathBuf::from(out_dir);
        }
        match self.path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => env::temp_dir(),
        }
    }
}

/// A private temporary directory for a single Tailwind CLI run, removed when
/// dropped.
#[derive(Debug)]
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Create a uniquely named directory inside `root`.
    fn new(root: &Path) -> Result<Self> {
        // The counter keeps names unique within a build script, and the process
        // id keeps them unique across the build scripts that share a `root`
        // under the Cargo target directory.
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = root.join(format!(
            "tailwind-scratch-{pid}-{n}",
            pid = std::process::id(),
            n = COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&path).map_err(|source| BuildError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Self { path })
    }

    /// The path of the created directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_dirs_get_distinct_paths() {
        let first = ScratchDir::new(&env::temp_dir()).unwrap();
        let second = ScratchDir::new(&env::temp_dir()).unwrap();
        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir());
        assert!(second.path().is_dir());
    }

    #[test]
    fn scratch_dir_is_removed_on_drop() {
        let scratch = ScratchDir::new(&env::temp_dir()).unwrap();
        let path = scratch.path().to_path_buf();
        assert!(path.is_dir());
        drop(scratch);
        assert!(!path.exists());
    }
}
