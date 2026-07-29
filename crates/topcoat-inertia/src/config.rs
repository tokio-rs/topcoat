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

#[cfg(all(test, feature = "asset"))]
mod tests {
    use std::fmt::Write as _;

    use topcoat_asset::{AssetCatalog, Manifest};
    use topcoat_view::View;

    use super::*;

    fn catalog(entries: &[(&str, u64)]) -> AssetCatalog {
        let mut source = "version = 1\n".to_owned();
        for (file, id) in entries {
            write!(
                &mut source,
                "\n[[assets]]\nid = {id}\nfile = \"{file}\"\nhash = \"hash\"\ncontent_type = \"text/plain\"\n"
            )
            .unwrap();
        }
        Manifest::parse(&source).unwrap().into()
    }

    fn version(catalog: &AssetCatalog) -> String {
        InertiaConfig::new(|_, _| View::empty())
            .version_from_assets(catalog)
            .version
            .unwrap()
    }

    #[test]
    fn asset_version_is_independent_of_catalog_iteration_order() {
        let forward = catalog(&[("app-a.js", 1), ("app-b.css", 2)]);
        let reverse = catalog(&[("app-b.css", 2), ("app-a.js", 1)]);

        assert_eq!(version(&forward), version(&reverse));
    }

    #[test]
    fn asset_filename_changes_the_version() {
        let before = catalog(&[("app-a.js", 1)]);
        let after = catalog(&[("app-b.js", 1)]);

        assert_ne!(version(&before), version(&after));
    }

    #[test]
    fn empty_catalog_has_a_stable_version() {
        let first = AssetCatalog::default();
        let second = AssetCatalog::default();

        assert_eq!(version(&first), version(&second));
        assert_eq!(version(&first).len(), 64);
    }
}
