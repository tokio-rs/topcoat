// Without the `serve` feature the docs' links to the serving configuration
// cannot resolve; they degrade to plain text instead of failing the build.
#![cfg_attr(not(feature = "serve"), allow(rustdoc::broken_intra_doc_links))]

#[cfg(feature = "serve")]
use std::collections::HashSet;

use topcoat_router::RouterBuilder;

#[cfg(feature = "serve")]
use crate::AssetRoute;
use crate::{AssetConfig, config::Host};

/// Registers assets on a [`RouterBuilder`].
///
/// Implemented for [`RouterBuilder`] so it is in scope wherever a router is
/// being built, enabling the [`assets`](Self::assets) method.
pub trait RouterBuilderAssetExt {
    /// Registers an [`AssetConfig`] on the router.
    ///
    /// The configuration is registered with the app context, where
    /// [`asset_config`](crate::asset_config) and
    /// [`bundled_asset`](crate::bundled_asset) read it back, and
    /// [`Asset`](crate::Asset) handles used as attribute values in the `view!`
    /// macro get rendered as the URL the asset is hosted at.
    ///
    /// A serving configuration ([`AssetConfig::serve`](crate::AssetConfig::serve))
    /// also adds an HTTP route for each bundled file, served by the
    /// application itself. Several assets can resolve to the same bundled
    /// file, in which case they share one route. A configuration hosted
    /// externally ([`AssetConfig::hosted_at`](crate::AssetConfig::hosted_at))
    /// adds no routes; the bundled files must be hosted at the configured
    /// base URL by other means.
    ///
    /// Anything convertible into an [`AssetConfig`] is accepted: an
    /// [`AssetBundle`](crate::AssetBundle) registers as the configuration
    /// serving it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[topcoat::router::page("/")] async fn about() -> topcoat::Result { topcoat::view::view! {} }
    /// use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .page(about)
    ///         .assets(AssetBundle::load().unwrap())
    ///         .build()
    /// }
    /// ```
    ///
    /// Hosting the bundled files on a CDN instead of serving them:
    ///
    /// ```rust
    /// use topcoat::asset::{AssetBundle, AssetConfig, RouterBuilderAssetExt};
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .assets(AssetConfig::hosted_at(
    ///             "https://cdn.example.com/assets",
    ///             AssetBundle::load().unwrap(),
    ///         ))
    ///         .build()
    /// }
    /// ```
    #[must_use]
    fn assets(self, config: impl Into<AssetConfig>) -> Self;
}

impl RouterBuilderAssetExt for RouterBuilder {
    fn assets(self, config: impl Into<AssetConfig>) -> Self {
        let config = config.into();

        let builder = match &config.host {
            #[cfg(feature = "serve")]
            Host::Serve { dir } => {
                let mut builder = self;
                let mut registered = HashSet::new();
                for asset in config.catalog.assets() {
                    if registered.insert(asset.name()) {
                        builder = builder.route(AssetRoute::new(dir, asset));
                    }
                }
                builder
            }
            Host::External { .. } => self,
        };

        builder.app_context(config)
    }
}

#[cfg(all(test, feature = "serve"))]
mod tests {
    use std::{env, fmt::Write, fs, path::PathBuf};

    use http::{Method, StatusCode, header::CONTENT_TYPE};
    use topcoat_router::{Body, Router};

    use super::*;
    use crate::{AssetBundle, AssetId, AssetOptions, Manifest, serve::ASSET_ROUTE_PREFIX};

    const OPTIONS: AssetOptions = AssetOptions::NONE;
    const MAIN: AssetId = AssetId::new("app", "src/main.rs", "assets/mark.svg", &OPTIONS);
    const OTHER: AssetId = AssetId::new("app", "src/other.rs", "assets/mark.svg", &OPTIONS);
    const FILE: &str = "mark-16313d41ffdefca4.svg";

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("topcoat-asset-router-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bundle(dir: PathBuf, entries: &[(AssetId, &str)]) -> AssetBundle {
        let mut toml = String::from("version = 1\n");
        for (id, content_type) in entries {
            let id = id.as_u64();
            let _ = write!(
                toml,
                "\n[[assets]]\nid = {id}\nfile = \"{FILE}\"\nhash = \"0\"\ncontent_type = \"{content_type}\"\n"
            );
        }
        AssetBundle {
            dir,
            catalog: Manifest::parse(&toml).unwrap().into(),
        }
    }

    fn get(router: &Router, path: &str) -> http::response::Parts {
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(router.handle(request));
        response.into_parts().0
    }

    #[test]
    fn one_file_declared_at_two_call_sites_serves_one_route() {
        let dir = temp_dir("shared-file");
        fs::write(dir.join(FILE), "<svg></svg>").unwrap();
        let bundle = bundle(dir, &[(MAIN, "image/svg+xml"), (OTHER, "image/svg+xml")]);

        let router = Router::builder().assets(bundle).build();

        let parts = get(&router, &format!("{ASSET_ROUTE_PREFIX}/{FILE}"));
        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(parts.headers.get(CONTENT_TYPE).unwrap(), "image/svg+xml");
    }

    #[test]
    fn a_catalog_that_disagrees_on_content_type_still_serves_one_route() {
        let dir = temp_dir("conflicting-content-types");
        fs::write(dir.join(FILE), "<svg></svg>").unwrap();
        let bundle = bundle(dir, &[(MAIN, "image/svg+xml"), (OTHER, "text/plain")]);

        let router = Router::builder().assets(bundle).build();

        let parts = get(&router, &format!("{ASSET_ROUTE_PREFIX}/{FILE}"));
        assert_eq!(parts.status, StatusCode::OK);
        let content_type = parts.headers.get(CONTENT_TYPE).unwrap();
        assert!(content_type == "image/svg+xml" || content_type == "text/plain");
    }
}
