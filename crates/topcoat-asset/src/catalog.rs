use std::collections::HashMap;

use crate::{AssetBundle, AssetId, Manifest};

/// A single entry inside an [`AssetCatalog`].
#[derive(Debug, Clone)]
pub struct BundledAsset {
    file: String,
    content_type: String,
}

impl BundledAsset {
    /// Bundled filename (typically `stem-<short-hash>.ext`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.file
    }

    /// `Content-Type` the asset is served with, resolved when the bundle was
    /// built.
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

/// The mapping from [`AssetId`]s to their bundled filenames and content
/// types.
///
/// This is the part of an asset bundle needed to resolve asset URLs, without
/// the bundled files themselves. An [`AssetConfig`](crate::AssetConfig)
/// carries the catalog its asset URLs resolve through.
///
/// An [`AssetBundle`](crate::AssetBundle) carries the catalog of the bundle
/// directory it loaded, and a [`Manifest`] converts into the catalog it
/// describes, for setups where only the manifest is available (e.g. embedded
/// into the binary on targets without filesystem access, like WebAssembly).
#[derive(Debug, Default, Clone)]
pub struct AssetCatalog {
    bundled_assets: HashMap<AssetId, BundledAsset>,
}

impl AssetCatalog {
    /// Look up the bundled asset for an [`AssetId`].
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&BundledAsset> {
        self.bundled_assets.get(&id)
    }

    /// Iterate over every bundled asset in arbitrary order.
    pub fn assets(&self) -> impl Iterator<Item = &BundledAsset> {
        self.bundled_assets.values()
    }
}

/// A bundle's catalog, dropping the bundle directory.
impl From<AssetBundle> for AssetCatalog {
    fn from(bundle: AssetBundle) -> Self {
        bundle.catalog
    }
}

/// Builds the catalog a manifest describes.
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
