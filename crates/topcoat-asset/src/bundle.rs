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
    /// Loads the bundle scoped to the current process executable.
    ///
    /// Deploy this directory alongside the binary from the same build.
    /// Use [`load_dir`](Self::load_dir) for a custom location.
    ///
    /// When called from a dynamically loaded library, this still loads the
    /// host process's bundle. To load a library's bundle, pass its artifact
    /// path to [`dir_for_artifact`](Self::dir_for_artifact) and then to
    /// [`load_dir`](Self::load_dir).
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if the bundle manifest is missing.
    /// Other errors can occur when locating the executable or loading the
    /// manifest through [`load_dir`](Self::load_dir).
    pub fn load() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let dir = Self::dir_for_artifact(exe)?;

        if !dir.join(MANIFEST_NAME).is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no asset bundle at {}", dir.display()),
            ));
        }

        Self::load_dir(dir)
    }

    /// Returns the default bundle directory for an executable or library artifact.
    ///
    /// The path is `assets/<artifact-file-name>` beside the artifact,
    /// including its extension when it has one. This keeps bundles for
    /// binaries and libraries in a shared directory independent.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if the artifact has no parent
    /// directory or file name.
    pub fn dir_for_artifact(artifact: impl AsRef<Path>) -> io::Result<PathBuf> {
        let artifact = artifact.as_ref();
        let parent = artifact.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "artifact has no parent directory")
        })?;
        let name = artifact
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "artifact has no file name"))?;

        Ok(parent.join("assets").join(name))
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
