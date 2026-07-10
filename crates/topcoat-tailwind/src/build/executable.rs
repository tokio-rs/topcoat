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

    /// Download the Tailwind CLI for `version` into the shared Topcoat cache,
    /// reusing the cached copy without downloading when it is already present.
    ///
    /// The binary is cached at
    /// `topcoat/cache/tailwind/tailwindcss-<version>-<platform>` inside the
    /// Cargo target directory, so it is shared across the workspace and reused
    /// by later builds even after a package's build fingerprint changes. Build
    /// scripts racing to download the same version are serialized with an
    /// exclusive file lock so only one downloads while the others wait and
    /// reuse its result. When `checksum` is given, the download's hash is
    /// verified against it before the file is moved into place; `checksum` must
    /// carry a supported algorithm prefix (`sha256:`). On Unix the file is made
    /// executable.
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

        // Fast path: a previous build already cached a verified copy. `dest`
        // only ever appears via an atomic rename of a fully downloaded and
        // checksum-verified file, so its mere existence means it is complete.
        if dest.exists() {
            return Ok(Executable::new(dest));
        }

        fs::create_dir_all(&dir).map_err(|source| BuildError::Io {
            path: dir.clone(),
            source,
        })?;

        // Serialize the download across processes. Cargo runs build scripts
        // concurrently, so several packages can reach here at once for the same
        // version; without this they would all download and race to rename over
        // `dest`. Holding an exclusive lock on a sibling lock file lets the
        // first process download while the rest block, then find the cached
        // copy below. The lock is advisory, but every code path that writes
        // `dest` goes through here, and the OS drops the lock if a holder dies,
        // so a crash can never wedge later builds. It is held until this
        // function returns.
        //
        // The lock file is intentionally left on disk and never removed:
        // `lock` acts on the file's inode, so unlinking it would let a waiter
        // and a newcomer that recreates the path lock two different inodes and
        // both proceed. It is an empty marker; once `dest` exists the fast path
        // above returns before it is even opened.
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

    /// The host platform suffix baked into cache file names, e.g.
    /// `macos-arm64` or `windows-x64.exe`. Derived from
    /// [`asset_name`](Self::asset_name), which every supported platform
    /// prefixes with `tailwindcss-`.
    fn platform() -> Result<&'static str> {
        let asset = Self::asset_name()?;
        Ok(asset.strip_prefix("tailwindcss-").unwrap_or(asset))
    }

    /// The directory the downloaded CLI is cached in: the shared Topcoat cache
    /// (`topcoat/cache/tailwind`) under the Cargo target directory, falling
    /// back to `OUT_DIR` itself when the build runs outside Cargo's target
    /// layout.
    fn cache_dir() -> Result<PathBuf> {
        if let Some(dir) = topcoat_core::runtime::cache::cache_dir("tailwind") {
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
        // The standalone Tailwind CLI is a Bun single-file executable that
        // unpacks its embedded native modules (the Oxide scanner, Lightning
        // CSS, the file watcher) into the temporary directory and loads them
        // with `dlopen`. On filesystems that support `O_TMPFILE` each run gets
        // a private anonymous copy, but otherwise Bun falls back to named files
        // at deterministic paths derived from the binary. Two processes running
        // the same shared executable then race on those paths, and one can load
        // a module while another is still writing it -- which surfaces, for the
        // scanner, as its `Scanner` export being undefined. Give each run its
        // own temporary directory so the extraction paths never collide.
        // Rooting it next to the executable keeps it on a filesystem that
        // permits execution, which the system temporary directory may not.
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

    /// The directory a per-run [`ScratchDir`] is created under: Cargo's
    /// `OUT_DIR` when set, otherwise the directory holding the executable, and
    /// finally the system temporary directory. The first two sit under the
    /// Cargo target directory, which permits executing the native modules Bun
    /// unpacks there.
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
