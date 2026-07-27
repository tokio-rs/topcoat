// Without the `serve` feature the docs' links to the serving constructor
// cannot resolve; they degrade to plain text instead of failing the build.
#![cfg_attr(not(feature = "serve"), allow(rustdoc::broken_intra_doc_links))]

use std::fmt;
#[cfg(feature = "serve")]
use std::path::PathBuf;

use topcoat_core::context::{Cx, app_context};

#[cfg(feature = "serve")]
use crate::AssetBundle;
#[cfg(feature = "serve")]
use crate::serve::ASSET_ROUTE_PREFIX;
use crate::{Asset, AssetCatalog, BundledAsset};

/// Where the bundled assets are hosted.
#[derive(Debug, Clone)]
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
///
/// Registering places the configuration in the app context, where
/// [`asset_config`] reads it back: [`get`](Self::get) looks up an asset's
/// bundled file, and [`resolve`](Self::resolve) forms the URL it is hosted
/// at. [`Asset`] values rendered in a view resolve their URL through the same
/// configuration.
#[derive(Debug, Clone)]
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
    /// let config = AssetConfig::hosted_at("https://cdn.example.com/assets", manifest);
    /// ```
    #[must_use]
    pub fn hosted_at(base_url: impl Into<String>, assets: impl Into<AssetCatalog>) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            catalog: assets.into(),
            host: Host::External { base_url },
        }
    }

    /// The catalog mapping [`AssetId`](crate::AssetId)s to their bundled
    /// files.
    #[must_use]
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Look up the bundled file for an [`Asset`] in the catalog.
    #[must_use]
    pub fn get(&self, asset: Asset) -> Option<&BundledAsset> {
        self.catalog.get(asset.id())
    }

    /// The base URL asset URLs are formed against: the internal asset route
    /// prefix for a serving configuration, or the base URL passed to
    /// [`hosted_at`](Self::hosted_at). Never ends with a `/`.
    #[must_use]
    pub fn base_url(&self) -> &str {
        match &self.host {
            #[cfg(feature = "serve")]
            Host::Serve { .. } => ASSET_ROUTE_PREFIX,
            Host::External { base_url } => base_url,
        }
    }

    /// Writes the URL `asset` is hosted at, `{base_url}/{bundled-filename}`,
    /// into `write`.
    ///
    /// This is how [`Asset`] values render in views; [`resolve`](Self::resolve)
    /// returns the same URL as a `String`.
    ///
    /// # Errors
    ///
    /// Propagates errors from writing to `write`.
    ///
    /// # Panics
    ///
    /// Panics if the catalog does not contain `asset`.
    pub fn fmt_url(&self, asset: Asset, write: &mut dyn fmt::Write) -> fmt::Result {
        let Some(bundled) = self.get(asset) else {
            panic!("failed to resolve asset {asset:?} in the asset catalog");
        };
        write.write_str(self.base_url())?;
        write.write_str("/")?;
        write.write_str(bundled.name())
    }

    /// Returns the URL `asset` is hosted at, `{base_url}/{bundled-filename}`.
    ///
    /// # Panics
    ///
    /// Panics if the catalog does not contain `asset`.
    #[must_use]
    pub fn resolve(&self, asset: Asset) -> String {
        let mut url = String::new();
        let _ = self.fmt_url(asset, &mut url);
        url
    }
}

/// Converts a bundle into the configuration serving it from the application.
#[cfg(feature = "serve")]
impl From<AssetBundle> for AssetConfig {
    fn from(bundle: AssetBundle) -> Self {
        AssetConfig::serve(bundle)
    }
}

/// Returns the [`AssetConfig`] registered as app context for this context.
///
/// # Panics
///
/// Panics if no [`AssetConfig`] was registered.
#[must_use]
pub fn asset_config(cx: &Cx) -> &AssetConfig {
    app_context(cx)
}

/// Resolves an [`Asset`] to its [`BundledAsset`] in the context's
/// registered [`AssetConfig`].
///
/// # Panics
///
/// Panics if no [`AssetConfig`] was registered, or if its catalog does not
/// contain the given asset.
#[must_use]
pub fn bundled_asset(cx: &Cx, asset: Asset) -> &BundledAsset {
    match asset_config(cx).get(asset) {
        Some(asset) => asset,
        None => panic!("failed to resolve asset {asset:?} in app context's asset config"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetId, AssetOptions, ENCODED_ASSET_SIZE, Manifest, RawAsset};

    #[test]
    fn hosted_at_trims_trailing_slashes() {
        let config =
            AssetConfig::hosted_at("https://cdn.example.com/assets///", AssetCatalog::default());

        assert_eq!(config.base_url(), "https://cdn.example.com/assets");
    }

    #[test]
    fn resolves_urls_against_the_base_url() {
        const OPTIONS: AssetOptions = AssetOptions::NONE;
        const ID: AssetId = AssetId::new("app", "src/lib.rs", "logo.png", &OPTIONS);
        static ENCODED: [u8; ENCODED_ASSET_SIZE] =
            RawAsset::encode(ID, "logo.png", "app", "/app", "src/lib.rs", &OPTIONS);
        let asset = Asset::new(&ENCODED);

        let manifest = Manifest::parse(&format!(
            r#"
version = 1

[[assets]]
id = {}
file = "logo-1a2b3c4d5e6f7a8b.png"
hash = "0"
content_type = "image/png"
"#,
            ID.as_u64()
        ))
        .unwrap();

        let config = AssetConfig::hosted_at("https://cdn.example.com/assets", manifest);

        assert_eq!(
            config.resolve(asset),
            "https://cdn.example.com/assets/logo-1a2b3c4d5e6f7a8b.png"
        );
    }
}
