use std::borrow::Cow;
use std::fmt::Write as _;

#[cfg(feature = "asset")]
use sha2::{Digest, Sha256};
use topcoat_core::context::Cx;
use topcoat_core::error::Result;
use topcoat_view::View;

#[cfg(feature = "asset")]
use topcoat_asset::AssetCatalog;

use crate::{CookieFlashStore, FlashStore, Page, Props};

pub trait ShareProps: Send + Sync {
    /// Adds shared props for the current page render.
    ///
    /// # Errors
    ///
    /// Returns an error when a shared value cannot be prepared.
    fn share<'cx>(&self, cx: &'cx Cx, props: &mut Props<'cx>) -> Result<()>;
}

impl<F> ShareProps for F
where
    F: for<'cx> Fn(&'cx Cx, &mut Props<'cx>) -> Result<()> + Send + Sync,
{
    fn share<'cx>(&self, cx: &'cx Cx, props: &mut Props<'cx>) -> Result<()> {
        self(cx, props)
    }
}

type Root = dyn Fn(&Cx, &Page) -> View + Send + Sync;
type Nonce = dyn Fn(&Cx) -> Option<String> + Send + Sync;

pub struct InertiaConfig {
    pub(crate) root: Box<Root>,
    pub(crate) version: Option<String>,
    pub(crate) root_id: Cow<'static, str>,
    pub(crate) nonce: Option<Box<Nonce>>,
    pub(crate) encrypt_history: bool,
    pub(crate) shared: Vec<Box<dyn ShareProps>>,
    pub(crate) flash_store: Box<dyn FlashStore>,
    pub(crate) convert_external_redirects: bool,
}

impl InertiaConfig {
    #[must_use]
    pub fn new(root: impl Fn(&Cx, &Page) -> View + Send + Sync + 'static) -> Self {
        Self {
            root: Box::new(root),
            version: None,
            root_id: Cow::Borrowed("app"),
            nonce: None,
            encrypt_history: false,
            shared: Vec::new(),
            flash_store: Box::new(CookieFlashStore::new()),
            convert_external_redirects: false,
        }
    }

    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    #[must_use]
    pub fn root_id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.root_id = id.into();
        self
    }

    #[must_use]
    pub fn nonce_with(
        mut self,
        resolve: impl Fn(&Cx) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        self.nonce = Some(Box::new(resolve));
        self
    }

    #[must_use]
    pub fn share_with<F>(mut self, share: F) -> Self
    where
        F: for<'cx> Fn(&'cx Cx, &mut Props<'cx>) -> Result<()> + Send + Sync + 'static,
    {
        self.shared.push(Box::new(share));
        self
    }

    #[must_use]
    pub fn encrypt_history(mut self) -> Self {
        self.encrypt_history = true;
        self
    }

    #[must_use]
    pub fn flash_store(mut self, store: impl FlashStore + 'static) -> Self {
        self.flash_store = Box::new(store);
        self
    }

    #[must_use]
    pub fn convert_external_redirects(mut self, enabled: bool) -> Self {
        self.convert_external_redirects = enabled;
        self
    }

    #[cfg(feature = "asset")]
    #[must_use]
    pub fn version_from_assets(mut self, catalog: &AssetCatalog) -> Self {
        let mut names = catalog
            .assets()
            .map(topcoat_asset::BundledAsset::name)
            .collect::<Vec<_>>();
        names.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

        let mut hasher = Sha256::new();
        for name in names {
            hasher.update(name.as_bytes());
            hasher.update([0]);
        }
        let mut version = String::with_capacity(64);
        for byte in hasher.finalize() {
            write!(&mut version, "{byte:02x}").expect("writing to a String cannot fail");
        }
        self.version = Some(version);
        self
    }
}
