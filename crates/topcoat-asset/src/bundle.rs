use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{AssetCatalog, AssetId, BundledAsset, MANIFEST_NAME, Manifest};

/// An asset bundle on disk: a directory of bundled files, together with the
/// [`AssetCatalog`] that maps [`AssetId`]s to them.
///
/// The [`Bundler`](crate::Bundler) (usually run through the `topcoat` CLI)
/// writes the bundle. Load it at runtime with [`AssetBundle::load`] or
/// [`AssetBundle::load_dir`], then register it on the router.
#[derive(Debug, Clone)]
pub struct AssetBundle {
    pub(crate) dir: PathBuf,
    pub(crate) catalog: AssetCatalog,
}

impl AssetBundle {
    /// Loads the bundle in the `assets` directory next to the current
    /// executable.
    ///
    /// By default the `topcoat` CLI writes the bundle to an `assets`
    /// directory next to the executable it scanned. With `cargo run`, this
    /// is `target/<profile>/assets`. When you deploy, ship that directory
    /// next to the binary.
    ///
    /// Use [`AssetBundle::load_dir`] when the bundle is somewhere else, for
    /// example a directory passed to `topcoat asset bundle --out`.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::NotFound`] if there is no
    /// manifest in that directory. Returns the same errors as
    /// [`AssetBundle::load_dir`] if the manifest cannot be read.
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

    /// Loads the bundle in the directory `dir`.
    ///
    /// `dir` is the bundle directory itself, the one that contains
    /// `manifest.toml` and the bundled files. A relative path is relative to
    /// the working directory of the process, not to the Cargo package.
    ///
    /// Use this when you write the bundle to your own location, such as
    /// `dist/assets`. Use [`AssetBundle::load`] for the default location next
    /// to the executable.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest cannot be read or parsed, or if it
    /// has an unsupported version.
    pub fn load_dir(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let manifest = Manifest::load(dir.join(MANIFEST_NAME))?;
        Ok(Self {
            dir,
            catalog: manifest.into(),
        })
    }

    /// Returns the directory the bundle was loaded from.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the catalog that maps [`AssetId`]s to the files in the bundle
    /// directory.
    #[must_use]
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Looks up the bundled file for an [`AssetId`].
    ///
    /// The file is located at [`dir`](AssetBundle::dir) joined with the
    /// entry's [`name`](BundledAsset::name).
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&BundledAsset> {
        self.catalog.get(id)
    }
}
