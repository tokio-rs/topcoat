use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{AssetCatalog, AssetId, BundledAsset, MANIFEST_NAME, Manifest};

/// An asset bundle on disk: a directory of bundled files plus the
/// [`AssetCatalog`] mapping [`AssetId`]s to them.
///
/// Built by the [`Bundler`](crate::Bundler) and loaded at runtime via
/// [`AssetBundle::load`] or [`AssetBundle::load_dir`].
#[derive(Debug, Clone)]
pub struct AssetBundle {
    pub(crate) dir: PathBuf,
    pub(crate) catalog: AssetCatalog,
}

impl AssetBundle {
    /// Load the bundle sitting next to the current executable.
    ///
    /// The bundler writes to an `assets` directory beside the executable it
    /// scanned, so `<exe_dir>/assets` is where a bundle belongs: `cargo run`
    /// finds `target/<profile>/assets`, and a deployment ships the directory
    /// alongside the binary.
    ///
    /// Use [`AssetBundle::load_dir`] when the bundle lives anywhere else, such
    /// as a custom path passed to the asset bundler with `--out`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if there is no readable manifest
    /// next to the executable, or propagates any I/O or parse error from
    /// reading it via [`AssetBundle::load_dir`].
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

        if !dir.join(MANIFEST_NAME).is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no asset bundle at {}", dir.display()),
            ));
        }

        Self::load_dir(dir)
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
    /// The returned entry names the file; its on-disk location is that name
    /// under [`dir`](AssetBundle::dir).
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&BundledAsset> {
        self.catalog.get(id)
    }
}
