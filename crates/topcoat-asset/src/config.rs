// Without the `serve` feature the docs' links to the serving constructor
// cannot resolve; they degrade to plain text instead of failing the build.
#![cfg_attr(not(feature = "serve"), allow(rustdoc::broken_intra_doc_links))]

#[cfg(feature = "serve")]
use std::path::PathBuf;

#[cfg(feature = "serve")]
use crate::AssetBundle;
use crate::AssetCatalog;

/// Where the bundled assets are hosted.
pub(crate) enum Host {
    /// Served by the application itself under the internal asset route
    /// prefix, reading files from this bundle directory.
    #[cfg(feature = "serve")]
    Serve { dir: PathBuf },
    /// Hosted externally; asset URLs are formed against this base URL.
    External { base_url: String },
}

/// Asset configuration, registered on the router (with the router's `assets`
/// extension method).
///
/// Built with [`AssetConfig::serve`], which serves a loaded
/// [`AssetBundle`](crate::AssetBundle)'s files from the application, or
/// [`AssetConfig::hosted_at`], which points asset URLs at an external host
/// instead. An [`AssetBundle`](crate::AssetBundle) also converts directly
/// into its serving configuration, so the common case registers as
/// `.assets(bundle)`.
pub struct AssetConfig {
    pub(crate) catalog: AssetCatalog,
    pub(crate) host: Host,
}

impl AssetConfig {
    /// Serves the bundle's files from the application.
    ///
    /// Each asset in the bundle is added as an HTTP route under the internal
    /// asset route prefix. This is the conversion used when an [`AssetBundle`]
    /// is registered directly, so `.assets(AssetConfig::serve(bundle))` and
    /// `.assets(bundle)` are equivalent.
    #[cfg(feature = "serve")]
    #[must_use]
    pub fn serve(bundle: AssetBundle) -> Self {
        let AssetBundle { dir, catalog } = bundle;
        Self {
            catalog,
            host: Host::Serve { dir },
        }
    }

    /// Hosts the bundled assets externally at `base_url` instead of serving
    /// them from the application.
    ///
    /// No asset routes are registered: the files described by `assets` must
    /// be made available under `base_url` by other means, such as a CDN or
    /// the reverse proxy in front of the application. Each asset's URL is
    /// `{base_url}/{bundled-filename}`; a trailing `/` on `base_url` is
    /// ignored. Bundled filenames are content-hashed, so the files can be
    /// served with long-lived, immutable caching.
    ///
    /// `assets` is anything that converts into an [`AssetCatalog`]: a loaded
    /// [`AssetBundle`](crate::AssetBundle), or a [`Manifest`](crate::Manifest)
    /// embedded into the binary on targets without filesystem access, such as
    /// WebAssembly:
    ///
    /// ```
    /// use topcoat::asset::{AssetConfig, Manifest};
    ///
    /// let manifest = Manifest::parse("version = 1\nassets = []").unwrap();
    /// let config = AssetConfig::hosted_at(manifest, "https://cdn.example.com/assets");
    /// ```
    #[must_use]
    pub fn hosted_at(assets: impl Into<AssetCatalog>, base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            catalog: assets.into(),
            host: Host::External { base_url },
        }
    }
}

/// Converts a bundle into the configuration serving it from the application.
#[cfg(feature = "serve")]
impl From<AssetBundle> for AssetConfig {
    fn from(bundle: AssetBundle) -> Self {
        AssetConfig::serve(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_at_trims_trailing_slashes() {
        let config =
            AssetConfig::hosted_at(AssetCatalog::default(), "https://cdn.example.com/assets///");

        match config.host {
            Host::External { base_url } => assert_eq!(base_url, "https://cdn.example.com/assets"),
            #[cfg(feature = "serve")]
            Host::Serve { .. } => panic!("expected an external host"),
        }
    }
}
