use std::borrow::Cow;

#[cfg(feature = "asset")]
use topcoat_asset::{Asset, asset_config};
use topcoat_core::{
    base_url::base_url,
    context::Cx,
    url_form::{UrlForm, url_form},
};
#[cfg(feature = "router")]
use topcoat_router::{HrefTarget, href, request::uri};

/// A URL in a page's metadata, such as its canonical URL or a social image.
///
/// Call [`resolve`](Self::resolve) to get the URL for a request, in the
/// [form](url_form) set on the context.
#[derive(Clone)]
#[non_exhaustive]
pub enum Url {
    /// A URL that is the same for every request. It is either absolute or
    /// relative to the site root, like `/images/logo.png`.
    Str(Cow<'static, str>),
    /// The URL of the current request, without its query string.
    #[cfg(feature = "router")]
    Current,
    /// The URL of a page or route without path parameters.
    #[cfg(feature = "router")]
    Route(&'static (dyn HrefTarget + Sync)),
    /// The URL of a bundled asset.
    #[cfg(feature = "asset")]
    Asset(Asset),
}

impl Url {
    /// Creates the URL of a page or route without path parameters, as in
    /// `Url::route(&about)`.
    #[cfg(feature = "router")]
    #[must_use]
    pub fn route(target: &'static (dyn HrefTarget + Sync)) -> Self {
        Self::Route(target)
    }

    /// Returns the URL for the request of `cx`, in the [form](url_form) set
    /// on `cx`.
    ///
    /// In the absolute form, a URL relative to the site root is joined with
    /// the [base URL](base_url) of the request. A URL that is already
    /// absolute is returned unchanged in either form.
    ///
    /// # Panics
    ///
    /// Panics if the form is absolute and `cx` has no base URL, or if the URL
    /// names a route with path parameters or an asset missing from the asset
    /// configuration.
    #[must_use]
    pub fn resolve<'a>(&'a self, cx: &'a Cx) -> Cow<'a, str> {
        match self {
            Self::Str(url) => in_form(cx, Cow::Borrowed(url)),
            #[cfg(feature = "router")]
            Self::Current => in_form(cx, Cow::Borrowed(uri(cx).path())),
            #[cfg(feature = "router")]
            Self::Route(target) => Cow::Owned(href(*target, ()).resolve(cx)),
            #[cfg(feature = "asset")]
            Self::Asset(asset) => in_form(cx, Cow::Owned(asset_config(cx).resolve(*asset))),
        }
    }
}

impl From<&'static str> for Url {
    fn from(url: &'static str) -> Self {
        Self::Str(Cow::Borrowed(url))
    }
}

impl From<String> for Url {
    fn from(url: String) -> Self {
        Self::Str(Cow::Owned(url))
    }
}

impl From<Cow<'static, str>> for Url {
    fn from(url: Cow<'static, str>) -> Self {
        Self::Str(url)
    }
}

#[cfg(feature = "asset")]
impl From<Asset> for Url {
    fn from(asset: Asset) -> Self {
        Self::Asset(asset)
    }
}

/// Returns `url` in the form set on `cx`: joined with the base URL when the
/// form is absolute and `url` is relative to the site root, and unchanged
/// otherwise.
fn in_form<'a>(cx: &Cx, url: Cow<'a, str>) -> Cow<'a, str> {
    match url_form(cx) {
        // `//host/path` is relative to the scheme only, not to the site root.
        UrlForm::Absolute if url.starts_with('/') && !url.starts_with("//") => {
            Cow::Owned(base_url(cx).join(&url))
        }
        _ => url,
    }
}
