use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{AssetCatalog, AssetId, BundledAsset, MANIFEST_NAME, Manifest};

/// A directory of bundled files and the catalog used to look them up.
///
/// Built by the [`Bundler`](crate::Bundler) and loaded at runtime via
/// [`AssetBundle::load`] or [`AssetBundle::load_dir`].
#[derive(Debug, Clone)]
pub struct AssetBundle {
    pub(crate) dir: PathBuf,
    pub(crate) catalog: AssetCatalog,
}

impl AssetBundle {
    /// Loads the `assets` directory next to the current executable.
    ///
    /// Deploy this directory alongside the binary from the same build.
    /// Use [`load_dir`](Self::load_dir) for a custom location.
    ///
    /// When the app was bundled with a shared target directory
    /// (`CARGO_TARGET_DIR`), the CLI writes the bundle into the
    /// `assets/<exe-stem>/` subdirectory so several apps sharing the
    /// directory never collect each other's assets; that subdirectory is
    /// loaded when present. A flat `assets/manifest.toml` — the layout a
    /// project-local target directory produces — is loaded otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if neither layout is found.
    /// Other errors can occur when locating the executable or loading the
    /// manifest through [`load_dir`](Self::load_dir).
    pub fn load() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let dir = exe
            .parent()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "current executable has no parent directory",
                )
            })?
            .join("assets");

        // A shared target directory layout first (`assets/<exe-stem>/`),
        // then the flat layout.
        if let Some(stem) = exe.file_stem().and_then(|stem| stem.to_str()) {
            let app_dir = dir.join(stem);
            if app_dir.join(MANIFEST_NAME).is_file() {
                return Self::load_dir(app_dir);
            }
        }
        if dir.join(MANIFEST_NAME).is_file() {
            return Self::load_dir(dir);
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no asset bundle at {}", dir.display()),
        ))
    }

    /// Load a bundle from a specific directory.
    ///
    /// `dir` must contain `manifest.toml` and the bundled files. Relative paths
    /// start from the process working directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be read or parsed, or if it
    /// reports an unsupported version.
    pub fn load_dir(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let manifest = Manifest::load(dir.join(MANIFEST_NAME))?;
        Ok(Self {
            dir,
            catalog: manifest.into(),
        })
    }

    /// Directory the bundle was loaded from.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The catalog mapping [`AssetId`]s to the files in the bundle directory.
    #[must_use]
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Look up the bundled file for an [`AssetId`].
    ///
    /// Join the returned filename to [`dir`](Self::dir) to locate it on disk.
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&BundledAsset> {
        self.catalog.get(id)
    }
}
