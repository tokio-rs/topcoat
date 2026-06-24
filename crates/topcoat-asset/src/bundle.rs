use std::{
    collections::HashMap,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

use crate::{Asset, MANIFEST_NAME, Manifest};

/// A single entry inside an [`AssetBundle`].
#[derive(Debug, Clone)]
pub struct BundledAsset {
    path: PathBuf,
    content_type: String,
}

impl BundledAsset {
    /// Absolute path to the bundled file on disk.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Bundled filename (typically `stem-<short-hash>.ext`).
    ///
    /// # Panics
    ///
    /// Panics if the bundled file path has no final component (e.g. it
    /// resolves to `/`), which should never happen for a bundle built by
    /// the [`Bundler`](crate::Bundler).
    #[must_use]
    pub fn name(&self) -> &OsStr {
        self.path
            .file_name()
            .expect("asset file path must have a name")
    }

    /// `Content-Type` the asset is served with, resolved when the bundle was
    /// built.
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

/// A loaded asset bundle: a directory of files plus the mapping from
/// [`Asset`] IDs to those files.
///
/// Built by the [`Bundler`](crate::Bundler) and loaded at runtime via
/// [`AssetBundle::load`] or [`AssetBundle::load_dir`].
#[derive(Debug, Default, Clone)]
pub struct AssetBundle {
    dir: PathBuf,
    bundled_assets: HashMap<Asset, BundledAsset>,
}

impl AssetBundle {
    /// Bundle with no assets and no directory; useful as a placeholder.
    #[must_use]
    pub fn empty() -> Self {
        AssetBundle::default()
    }

    /// Auto-detect and load the bundle from a conventional location.
    ///
    /// Walks up from the current executable, checking each ancestor for an
    /// `assets/manifest.toml`. This covers, without configuration:
    ///
    /// - `<exe_dir>/assets/` — deployment, bundle shipped next to the binary.
    /// - `target/<profile>/<bin>` → `target/assets/` (typical `cargo run`).
    /// - `target/<profile>/examples/<bin>` → `target/assets/`.
    /// - `target/<triple>/<profile>/<bin>` → `target/assets/` (cross-compile).
    /// - `target/<triple>/<profile>/examples/<bin>` → `target/assets/`.
    ///
    /// The first directory that contains a readable `manifest.toml` is
    /// loaded. The walk stops once it reaches a directory named `target`
    /// (since `<target>/assets` is checked at that step) or after a bounded
    /// number of ancestors. Returns [`io::ErrorKind::NotFound`] if no
    /// candidate has a manifest.
    ///
    /// Use [`AssetBundle::load_dir`] when you already know the exact bundle
    /// directory, such as a custom path passed to the asset bundler.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no candidate `assets` directory
    /// contains a readable manifest, or propagates any I/O or parse error from
    /// reading the located manifest via [`AssetBundle::load_dir`].
    pub fn load() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "current executable has no parent directory",
                )
            })?
            .to_path_buf();

        let mut tried = Vec::new();
        let mut current = Some(exe_dir.as_path());
        for _ in 0..6 {
            let Some(d) = current else { break };
            let candidate = d.join("assets");
            if candidate.join(MANIFEST_NAME).is_file() {
                return Self::load_dir(candidate);
            }
            tried.push(candidate);
            if d.file_name() == Some(OsStr::new("target")) {
                break;
            }
            current = d.parent();
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no asset bundle found near {}: tried {}",
                exe_dir.display(),
                tried
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ))
    }

    /// Load a bundle from a specific directory.
    ///
    /// `dir` must be the asset bundle directory itself: the directory that
    /// contains `manifest.toml` and the bundled asset files. The path is
    /// resolved like any other filesystem path, so a relative path is relative
    /// to the process working directory, not to the Cargo package or workspace.
    ///
    /// This is useful when your application controls where bundles are written,
    /// for example `dist/assets` or another deployment-specific location. Use
    /// [`AssetBundle::load`] instead when you want Topcoat to look for a
    /// conventional `assets` directory near the current executable.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be read or parsed, or if it
    /// reports an unsupported version.
    pub fn load_dir(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let manifest = Manifest::load(dir.join(MANIFEST_NAME))?;

        let bundled_assets = manifest
            .assets
            .into_iter()
            .map(|entry| {
                (
                    entry.id,
                    BundledAsset {
                        path: dir.join(entry.file),
                        content_type: entry.content_type,
                    },
                )
            })
            .collect();

        Ok(Self {
            dir,
            bundled_assets,
        })
    }

    /// Directory the bundle was loaded from.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Look up the bundled file for an [`Asset`] ID.
    #[must_use]
    pub fn get(&self, id: Asset) -> Option<&BundledAsset> {
        self.bundled_assets.get(&id)
    }

    /// Iterate over every bundled asset in arbitrary order.
    pub fn assets(&self) -> impl Iterator<Item = &BundledAsset> {
        self.bundled_assets.values()
    }
}
