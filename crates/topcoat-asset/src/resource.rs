use topcoat_core::{
    context::Cx,
    error::Result,
    response_event::{ClientResource, ClientResourceKind},
};

use crate::{Asset, asset_config};

/// An asset the browser should load while the response is streaming.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetResource {
    /// A stylesheet loaded with a `<link rel="stylesheet">` element.
    Stylesheet(Asset),
    /// A JavaScript module loaded with a `<script type="module">` element.
    Module(Asset),
}

impl Asset {
    /// Loads this asset as a stylesheet while the response is streaming.
    #[must_use]
    pub const fn stylesheet(self) -> AssetResource {
        AssetResource::Stylesheet(self)
    }

    /// Loads this asset as a JavaScript module while the response is streaming.
    #[must_use]
    pub const fn module(self) -> AssetResource {
        AssetResource::Module(self)
    }
}

/// Adds streaming asset requirements to a request context.
pub trait CxAssetExt {
    /// Starts loading `resource` in the browser as soon as this requirement is
    /// flushed to the response. Repeated requirements are deduplicated.
    ///
    /// # Errors
    ///
    /// Returns an error if the same internal resource key is used for
    /// incompatible requirements.
    fn require_asset(&self, resource: AssetResource) -> Result<()>;
}

impl CxAssetExt for Cx {
    fn require_asset(&self, resource: AssetResource) -> Result<()> {
        let (asset, kind, kind_key) = match resource {
            AssetResource::Stylesheet(asset) => {
                (asset, ClientResourceKind::Stylesheet, "stylesheet")
            }
            AssetResource::Module(asset) => (asset, ClientResourceKind::Module, "module"),
        };
        let key = format!("asset:{kind_key}:{:016x}", asset.id().as_u64());
        let url = asset_config(self).resolve(asset);

        self.require_client_resource(ClientResource { key, kind, url })
    }
}
