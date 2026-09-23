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

/// Where the bundled files are hosted.
#[derive(Debug, Clone)]
pub(crate) enum Host {
    /// Served by the application itself under the internal asset route
    /// prefix, reading files from this bundle directory.
    #[cfg(feature = "serve")]
    Serve { dir: PathBuf },
    /// Hosted externally; asset URLs are formed against this base URL.
    External { base_url: String },
}

/// The asset configuration of a router: which bundled files exist and where
/// they are hosted.
///
/// Create one with [`AssetConfig::serve`] to serve the files of an
/// [`AssetBundle`](crate::AssetBundle) from the application, or with
/// [`AssetConfig::hosted_at`] to point asset URLs at an external host. An
/// [`AssetBundle`](crate::AssetBundle) converts into a serving configuration,
/// so in the common case you register it with `.assets(bundle)`.
///
/// Register the configuration with the router's `assets` method, which
/// stores it in the app context. Read it back with [`asset_config`]. An
/// [`Asset`] rendered in a view gets its URL from this configuration.
#[derive(Debug, Clone)]
pub struct AssetConfig {
    pub(crate) catalog: AssetCatalog,
    pub(crate) host: Host,
}

impl AssetConfig {
    /// Creates a configuration that serves the files of `bundle` from the
    /// application.
    ///
    /// Each bundled file gets a `GET` route under `/_topcoat/assets`.
    /// `.assets(AssetConfig::serve(bundle))` and `.assets(bundle)` do the
    /// same thing.
    #[cfg(feature = "serve")]
    #[must_use]
    pub fn serve(bundle: AssetBundle) -> Self {
        let AssetBundle { dir, catalog } = bundle;
        Self {
            catalog,
            host: Host::Serve { dir },
        }
    }

    /// Creates a configuration for bundled files hosted at `base_url`, instead
    /// of served by the application.
    ///
    /// The router adds no asset routes. You must upload the bundled files to
    /// `base_url` yourself, for example to a CDN or to the reverse proxy in
    /// front of the application. The URL of each asset is
    /// `{base_url}/{bundled-filename}`. Trailing slashes on `base_url` are
    /// removed. Bundled filenames contain a content hash, so the host can
    /// cache them forever.
    ///
    /// `assets` is anything that converts into an [`AssetCatalog`]: a loaded
    /// [`AssetBundle`](crate::AssetBundle), or a [`Manifest`](crate::Manifest).
    /// A manifest is useful on targets without filesystem access, such as
    /// WebAssembly, where you embed it into the binary:
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

    /// Returns the catalog that maps [`AssetId`](crate::AssetId)s to their
    /// bundled files.
    #[must_use]
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Looks up the bundled file for an [`Asset`].
    #[must_use]
    pub fn get(&self, asset: Asset) -> Option<&BundledAsset> {
        self.catalog.get(asset.id())
    }

    /// Returns the URL that asset URLs start with.
    ///
    /// This is `/_topcoat/assets` for a configuration created with
    /// [`serve`](Self::serve), and the base URL passed to
    /// [`hosted_at`](Self::hosted_at) otherwise. It never ends with a `/`.
    #[must_use]
    pub fn base_url(&self) -> &str {
        match &self.host {
            #[cfg(feature = "serve")]
            Host::Serve { .. } => ASSET_ROUTE_PREFIX,
            Host::External { base_url } => base_url,
        }
    }

    /// Writes the URL of `asset`, `{base_url}/{bundled-filename}`, into
    /// `write`.
    ///
    /// [`resolve`](Self::resolve) returns the same URL as a `String`.
    ///
    /// # Errors
    ///
    /// Returns any error from writing to `write`.
    ///
    /// # Panics
    ///
    /// Panics if the catalog does not contain `asset`.
    #[track_caller]
    pub fn fmt_url(&self, asset: Asset, write: &mut dyn fmt::Write) -> fmt::Result {
        let Some(bundled) = self.get(asset) else {
            panic!("failed to resolve asset {asset:?} in the asset catalog");
        };
        write.write_str(self.base_url())?;
        write.write_str("/")?;
        write.write_str(bundled.name())
    }

    /// Returns the URL of `asset`, `{base_url}/{bundled-filename}`.
    ///
    /// # Panics
    ///
    /// Panics if the catalog does not contain `asset`.
    #[must_use]
    #[track_caller]
    pub fn resolve(&self, asset: Asset) -> String {
        let mut url = String::new();
        let _ = self.fmt_url(asset, &mut url);
        url
    }
}

/// Creates a configuration that serves the bundle from the application, like
/// [`AssetConfig::serve`].
#[cfg(feature = "serve")]
impl From<AssetBundle> for AssetConfig {
    fn from(bundle: AssetBundle) -> Self {
        AssetConfig::serve(bundle)
    }
}

/// Returns the [`AssetConfig`] registered on the router.
///
/// # Panics
///
/// Panics if the router has no [`AssetConfig`].
#[must_use]
#[track_caller]
pub fn asset_config(cx: &Cx) -> &AssetConfig {
    app_context(cx)
}

/// Returns the bundled file of `asset` from the [`AssetConfig`] registered
/// on the router.
///
/// # Panics
///
/// Panics if the router has no [`AssetConfig`], or if its catalog does not
/// contain `asset`.
#[must_use]
#[track_caller]
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
