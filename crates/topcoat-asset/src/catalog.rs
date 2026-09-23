use std::collections::HashMap;

use crate::{AssetBundle, AssetId, Manifest};

/// One bundled file in an [`AssetCatalog`].
#[derive(Debug, Clone)]
pub struct BundledAsset {
    file: String,
    content_type: String,
}

impl BundledAsset {
    /// Returns the bundled filename, usually `stem-<hash>.ext`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.file
    }

    /// Returns the `Content-Type` the file is served with.
    ///
    /// The bundler decides this when it builds the bundle.
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

/// A map from [`AssetId`]s to their bundled files.
///
/// A catalog is all that is needed to build asset URLs. It does not need the
/// bundled files themselves. Get one from a loaded
/// [`AssetBundle`](crate::AssetBundle), or convert a [`Manifest`] into one
/// when only the manifest is available, for example when it is embedded
/// into a WebAssembly binary that has no filesystem access.
#[derive(Debug, Default, Clone)]
pub struct AssetCatalog {
    bundled_assets: HashMap<AssetId, BundledAsset>,
}

impl AssetCatalog {
    /// Looks up the bundled file for an [`AssetId`].
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&BundledAsset> {
        self.bundled_assets.get(&id)
    }

    /// Returns an iterator over all entries, in no particular order.
    ///
    /// Several IDs can map to the same bundled file, so a file can appear
    /// more than once.
    pub fn assets(&self) -> impl Iterator<Item = &BundledAsset> {
        self.bundled_assets.values()
    }
}

/// Takes the catalog of a bundle and drops the bundle directory.
impl From<AssetBundle> for AssetCatalog {
    fn from(bundle: AssetBundle) -> Self {
        bundle.catalog
    }
}

/// Builds the catalog that a manifest describes.
impl From<Manifest> for AssetCatalog {
    fn from(manifest: Manifest) -> Self {
        Self {
            bundled_assets: manifest
                .assets
                .into_iter()
                .map(|entry| {
                    (
                        entry.id,
                        BundledAsset {
                            file: entry.file,
                            content_type: entry.content_type,
                        },
                    )
                })
                .collect(),
        }
    }
}
