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
///
/// Its download is verified against a checksum that ships with Topcoat.
pub const DEFAULT_VERSION: &str = "4.3.2";

/// A standalone Tailwind CLI release asset and the host platform it runs on.
struct Asset {
    /// The host's [`std::env::consts::OS`].
    os: &'static str,
    /// The host's [`std::env::consts::ARCH`].
    arch: &'static str,
    /// The file name of the asset in a GitHub release.
    name: &'static str,
    /// The SHA-256 of the asset in the [`DEFAULT_VERSION`] release as lowercase hex, or `None`
    /// if that release does not publish the asset.
    default_sha256: Option<&'static str>,
}

/// Every asset Topcoat can download, by host platform.
///
/// When changing [`DEFAULT_VERSION`], replace each `default_sha256` with the asset's `digest` from
/// `gh api repos/tailwindlabs/tailwindcss/releases/tags/v<version>`.
const ASSETS: &[Asset] = &[
    Asset {
        os: "macos",
        arch: "x86_64",
        name: "tailwindcss-macos-x64",
        default_sha256: Some("cef8f110471e889c3c4409055cf8aff33076f58a081867b0dfc6534b290bfbb0"),
    },
    Asset {
        os: "macos",
        arch: "aarch64",
        name: "tailwindcss-macos-arm64",
        default_sha256: Some("b800b0659dc64b9f03ede5660244d9415d777d5739ae2889280877ca37be742a"),
    },
    Asset {
        os: "linux",
        arch: "x86_64",
        name: "tailwindcss-linux-x64",
        default_sha256: Some("5036c4fb4328e0bcdbb6065c70d8ac9452e0d4c947113a788a8f94fd390425c1"),
    },
    Asset {
        os: "linux",
        arch: "aarch64",
        name: "tailwindcss-linux-arm64",
        default_sha256: Some("394ddccc2402cfa3abd97dfba56f3587781a3d6e6ce66e65ceada14beb7664b8"),
    },
    Asset {
        os: "linux",
        arch: "arm",
        name: "tailwindcss-linux-armv7",
        default_sha256: None,
    },
    Asset {
        os: "windows",
        arch: "x86_64",
        name: "tailwindcss-windows-x64.exe",
        default_sha256: Some("224a62a8351d3b8da9d950a4eb1d7176dc901dc4735b47f816f3dfcbc67d8654"),
    },
    Asset {
        os: "windows",
        arch: "aarch64",
        name: "tailwindcss-windows-arm64.exe",
        default_sha256: None,
    },
];

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
        /// `"sha256:b800b065..."`. Verified once after download. `None`
        /// verifies the [`DEFAULT_VERSION`] release against a checksum that
        /// ships with Topcoat and skips verification for other releases.
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
    /// The download is checked against [`Self::expected_digest`] before it
    /// enters the cache. Cached files are reused without verification. On
    /// Unix, the downloaded file is made executable.
    fn download_from_github(version: &str, checksum: Option<&str>) -> Result<Executable> {
        // Resolve the digest up front so a malformed checksum fails before
        // anything is downloaded.
        let expected_digest = Self::expected_digest(version, checksum)?;

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
        // during the download, when a digest is known.
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
            name = Self::asset()?.name,
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

    /// The SHA-256 hex digest the download of `version` must match: the one in
    /// `checksum`, which must use the `sha256:` prefix, or the one Topcoat
    /// ships for the [`DEFAULT_VERSION`] release. `None` skips verification.
    fn expected_digest<'a>(version: &str, checksum: Option<&'a str>) -> Result<Option<&'a str>> {
        match checksum {
            Some(checksum) => checksum.strip_prefix("sha256:").map(Some).ok_or_else(|| {
                BuildError::UnsupportedChecksum {
                    checksum: checksum.to_owned(),
                }
            }),
            None if version == DEFAULT_VERSION => Ok(Self::asset()?.default_sha256),
            None => Ok(None),
        }
    }

    /// Returns the GitHub release asset for the host platform.
    fn asset() -> Result<&'static Asset> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        ASSETS
            .iter()
            .find(|asset| asset.os == os && asset.arch == arch)
            .ok_or(BuildError::UnsupportedPlatform { os, arch })
    }

    /// The host platform suffix used in cache filenames.
    fn platform() -> Result<&'static str> {
        let name = Self::asset()?.name;
        Ok(name.strip_prefix("tailwindcss-").unwrap_or(name))
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
    fn the_default_release_is_verified_against_the_shipped_digest() {
        let Ok(asset) = ExecutableSource::asset() else {
            return;
        };
        let digest = ExecutableSource::expected_digest(DEFAULT_VERSION, None).unwrap();
        assert_eq!(digest, asset.default_sha256);
    }

    #[test]
    fn other_releases_are_only_verified_against_a_given_checksum() {
        assert_eq!(
            ExecutableSource::expected_digest("4.0.0", None).unwrap(),
            None
        );
        assert_eq!(
            ExecutableSource::expected_digest("4.0.0", Some("sha256:abc")).unwrap(),
            Some("abc"),
        );
    }

    #[test]
    fn a_given_checksum_replaces_the_shipped_digest() {
        let digest = ExecutableSource::expected_digest(DEFAULT_VERSION, Some("sha256:abc"));
        assert_eq!(digest.unwrap(), Some("abc"));
    }

    #[test]
    fn a_checksum_without_the_sha256_prefix_is_rejected() {
        let error = ExecutableSource::expected_digest(DEFAULT_VERSION, Some("md5:abc"));
        assert!(matches!(
            error,
            Err(BuildError::UnsupportedChecksum { checksum }) if checksum == "md5:abc",
        ));
    }

    #[test]
    fn the_shipped_digests_are_distinct_sha256_hex() {
        let digests: Vec<_> = ASSETS
            .iter()
            .filter_map(|asset| asset.default_sha256)
            .collect();
        for digest in &digests {
            assert!(
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                "malformed digest {digest:?}",
            );
        }
        // Different binaries never share a digest, so a repeat is a copy and paste mistake.
        let mut unique = digests.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), digests.len(), "a digest is listed twice");
    }

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
