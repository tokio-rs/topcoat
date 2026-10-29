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
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, PartsWriter};

/// A metadata URL that points to a page or an image.
///
/// [`resolve`](Self::resolve) returns the URL for the current request. The
/// context's [URL form](url_form) controls whether it includes the site's
/// base URL.
#[derive(Clone)]
#[non_exhaustive]
pub enum Url {
    /// A fixed URL, either absolute or relative to the site root, such as
    /// `/images/logo.png`.
    Str(Cow<'static, str>),
    /// The current request's URL with the query string left out.
    #[cfg(feature = "router")]
    Current,
    /// A link to a page or route that has no path parameters.
    #[cfg(feature = "router")]
    Route(&'static (dyn HrefTarget + Sync)),
    /// A link to an asset in the bundle.
    #[cfg(feature = "asset")]
    Asset(Asset),
}

impl Url {
    /// Links to a page or route, for example `Url::route(&about)`.
    /// The target must have no path parameters.
    #[cfg(feature = "router")]
    #[must_use]
    pub fn route(target: &'static (dyn HrefTarget + Sync)) -> Self {
        Self::Route(target)
    }

    /// Builds the URL using the request and [URL form](url_form) in `cx`.
    ///
    /// When absolute URLs are requested, a path relative to the site root
    /// gets the [base URL](base_url) from `cx` added before it. URLs that
    /// are already absolute stay unchanged.
    ///
    /// # Panics
    ///
    /// Panics if resolving the URL needs a base URL that is missing from
    /// `cx`. Also panics if the target route has path parameters or the
    /// asset is missing from the asset configuration.
    #[must_use]
    pub fn resolve<'a>(&'a self, cx: &'a Cx) -> Cow<'a, str> {
        match self {
            Self::Str(url) => in_form(cx, Cow::Borrowed(url)),
            #[cfg(feature = "router")]
            Self::Current => in_form(cx, Cow::Borrowed(uri(cx).path())),
            #[cfg(feature = "router")]
            Self::Route(target) => Cow::Owned(href(*target, ()).resolve(cx)),
            #[cfg(feature = "asset")]
            Self::Asset(asset) => Cow::Owned(asset_config(cx).resolve(cx, *asset)),
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

/// Renders the URL in an HTML attribute using the view's [URL form](url_form).
/// For example, `<link rel="canonical" href=(canonical)>`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for &Url {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Url::Str(Cow::Borrowed(url)) => {
                push_base(cx, url, parts);
                parts.push_static_str(url);
            }
            Url::Str(Cow::Owned(url)) => {
                push_base(cx, url, parts);
                parts.push_str(url);
            }
            #[cfg(feature = "router")]
            Url::Current => {
                let path = uri(cx).path();
                push_base(cx, path, parts);
                parts.push_str(path);
            }
            #[cfg(feature = "router")]
            Url::Route(target) => {
                AttributeValueViewParts::into_view_parts(href(*target, ()), cx, parts);
            }
            #[cfg(feature = "asset")]
            Url::Asset(asset) => {
                AttributeValueViewParts::into_view_parts(*asset, cx, parts);
            }
        }
    }
}

/// Adds the base URL when `cx` requests absolute URLs and `url` is relative
/// to the site root. Otherwise, returns `url` unchanged.
fn in_form<'a>(cx: &Cx, url: Cow<'a, str>) -> Cow<'a, str> {
    if needs_base(cx, &url) {
        Cow::Owned(base_url(cx).join(&url))
    } else {
        url
    }
}

/// Writes the base URL into the view when `url` needs that prefix.
#[cfg(feature = "view")]
fn push_base(cx: &Cx, url: &str, parts: &mut PartsWriter<'_>) {
    if needs_base(cx, url) {
        parts.push_str(base_url(cx).as_str());
    }
}

/// Checks whether `url` needs a base URL to match the form requested by `cx`.
///
/// Only paths starting with a single `/` need the prefix. The base URL has
/// no trailing slash, so it can go directly before the path.
fn needs_base(cx: &Cx, url: &str) -> bool {
    // `//host/path` is relative to the scheme only, not to the site root.
    url_form(cx) == UrlForm::Absolute && url.starts_with('/') && !url.starts_with("//")
}
