//! The base URL an application is publicly reachable at.

use core::fmt;
use std::str::FromStr;

use crate::context::{Cx, try_app_context};

/// The absolute URL an application is publicly reachable at, like
/// `https://example.com`.
///
/// Relative URLs work anywhere within the site, but rendered content that
/// leaves it (e.g. links and images in emails, feeds, or sitemaps) needs
/// the absolute form, resolved against this base. A base URL is an `http` or
/// `https` URL with a host and an optional path prefix (for applications
/// mounted under one, like `https://example.com/app`), and no query or
/// fragment. The string is parsed at construction, so every value of this
/// type holds a well-formed base.
///
/// Register one on the router builder with `.base_url(...)`, read it back
/// with [`base_url`] or [`try_base_url`], and resolve paths against it with
/// [`join`](BaseUrl::join):
///
/// ```
/// use topcoat::context::BaseUrl;
///
/// let base = BaseUrl::new("https://example.com")?;
/// assert_eq!(base.join("/assets/logo.png"), "https://example.com/assets/logo.png");
/// # Ok::<(), topcoat::context::BaseUrlError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseUrl {
    /// The normalized base: lowercase scheme, authority, and path prefix,
    /// without a trailing slash.
    url: String,
}

impl BaseUrl {
    /// Parses a base URL.
    ///
    /// The scheme and any trailing slash are normalized, so
    /// `https://example.com/app/` and `https://example.com/app` are the
    /// same base.
    ///
    /// # Errors
    ///
    /// Returns [`BaseUrlError`] if the string is not an absolute `http` or
    /// `https` URL, or carries a query string or fragment.
    pub fn new(url: impl AsRef<str>) -> Result<BaseUrl, BaseUrlError> {
        let url = url.as_ref();
        // `http::Uri` silently discards fragments, so reject them upfront.
        if url.contains('#') {
            return Err(BaseUrlError::HasFragment);
        }
        let uri: http::Uri = url.parse().map_err(BaseUrlError::Invalid)?;
        let Some(scheme) = uri.scheme_str() else {
            return Err(BaseUrlError::NotAbsolute);
        };
        if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
            return Err(BaseUrlError::UnsupportedScheme);
        }
        let Some(authority) = uri.authority() else {
            return Err(BaseUrlError::NotAbsolute);
        };
        if uri.query().is_some() {
            return Err(BaseUrlError::HasQuery);
        }
        let path = uri.path().trim_end_matches('/');
        Ok(BaseUrl {
            url: format!("{}://{authority}{path}", scheme.to_ascii_lowercase()),
        })
    }

    /// Resolves a root-relative path into an absolute URL.
    ///
    /// The path is taken as relative to the application root whether or not
    /// it starts with a slash: `base.join("/assets/logo.png")` and
    /// `base.join("assets/logo.png")` produce the same URL. A query string
    /// on the path is carried through.
    #[must_use]
    pub fn join(&self, path: &str) -> String {
        format!("{}/{}", self.url, path.trim_start_matches('/'))
    }

    /// The base URL as a string, without a trailing slash.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }
}

impl fmt::Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.url)
    }
}

impl FromStr for BaseUrl {
    type Err = BaseUrlError;

    fn from_str(s: &str) -> Result<BaseUrl, BaseUrlError> {
        BaseUrl::new(s)
    }
}

impl TryFrom<&str> for BaseUrl {
    type Error = BaseUrlError;

    fn try_from(url: &str) -> Result<BaseUrl, BaseUrlError> {
        url.parse()
    }
}

impl TryFrom<String> for BaseUrl {
    type Error = BaseUrlError;

    fn try_from(url: String) -> Result<BaseUrl, BaseUrlError> {
        url.parse()
    }
}

impl TryFrom<&String> for BaseUrl {
    type Error = BaseUrlError;

    fn try_from(url: &String) -> Result<BaseUrl, BaseUrlError> {
        url.parse()
    }
}

impl From<&BaseUrl> for BaseUrl {
    fn from(base_url: &BaseUrl) -> BaseUrl {
        base_url.clone()
    }
}

/// The reason a string was rejected as a base URL.
#[derive(Debug, thiserror::Error)]
pub enum BaseUrlError {
    /// The string is not a valid URL.
    #[error("invalid base URL: {0}")]
    Invalid(#[source] http::uri::InvalidUri),
    /// The URL has no scheme or host, like `example.com/app` or `/app`.
    #[error("base URL must be absolute, like `https://example.com`")]
    NotAbsolute,
    /// The scheme is neither `http` nor `https`.
    #[error("base URL scheme must be `http` or `https`")]
    UnsupportedScheme,
    /// The URL carries a query string, which a base cannot have.
    #[error("base URL cannot have a query string")]
    HasQuery,
    /// The URL carries a fragment, which a base cannot have.
    #[error("base URL cannot have a fragment")]
    HasFragment,
}

/// Returns the [`BaseUrl`] registered on the router.
///
/// # Panics
///
/// Panics if no base URL has been registered. Register one on the router
/// builder with `.base_url(...)`.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, base_url};
///
/// fn logo_url(cx: &Cx) -> String {
///     base_url(cx).join("/assets/logo.png")
/// }
/// ```
#[must_use]
pub fn base_url(cx: &Cx) -> &BaseUrl {
    match try_base_url(cx) {
        Some(base_url) => base_url,
        None => panic!(
            "attempted to access the base URL, but none was registered; \
             register one on the router builder with `.base_url(...)`"
        ),
    }
}

/// Returns the [`BaseUrl`] registered on the router, or `None` if none has
/// been registered.
#[must_use]
pub fn try_base_url(cx: &Cx) -> Option<&BaseUrl> {
    try_app_context(cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CxTestBuilder;

    #[test]
    fn accepts_and_normalizes_absolute_http_urls() -> Result<(), BaseUrlError> {
        let base = BaseUrl::new("https://example.com")?;
        assert_eq!(base.as_str(), "https://example.com");
        assert_eq!(base.to_string(), "https://example.com");

        let dev: BaseUrl = "http://localhost:3000".parse()?;
        assert_eq!(dev.as_str(), "http://localhost:3000");

        let prefixed = BaseUrl::try_from("https://example.com/app/")?;
        assert_eq!(prefixed.as_str(), "https://example.com/app");

        let uppercase = BaseUrl::new("HTTPS://example.com/")?;
        assert_eq!(uppercase.as_str(), "https://example.com");

        Ok(())
    }

    #[test]
    fn rejects_urls_that_cannot_serve_as_a_base() {
        assert!(matches!(
            BaseUrl::new("example.com"),
            Err(BaseUrlError::NotAbsolute)
        ));
        assert!(matches!(
            BaseUrl::new("/app"),
            Err(BaseUrlError::NotAbsolute)
        ));
        assert!(matches!(
            BaseUrl::new("ftp://example.com"),
            Err(BaseUrlError::UnsupportedScheme)
        ));
        assert!(matches!(
            BaseUrl::new("https://example.com?page=1"),
            Err(BaseUrlError::HasQuery)
        ));
        assert!(matches!(
            BaseUrl::new("https://example.com#top"),
            Err(BaseUrlError::HasFragment)
        ));
        assert!(matches!(
            BaseUrl::new("https://exa mple.com"),
            Err(BaseUrlError::Invalid(_))
        ));
    }

    #[test]
    fn joins_paths_below_the_base() -> Result<(), BaseUrlError> {
        let base = BaseUrl::new("https://example.com")?;
        assert_eq!(
            base.join("/assets/logo.png"),
            "https://example.com/assets/logo.png"
        );
        assert_eq!(
            base.join("assets/logo.png"),
            "https://example.com/assets/logo.png"
        );
        assert_eq!(base.join("/posts?page=2"), "https://example.com/posts?page=2");
        assert_eq!(base.join("/"), "https://example.com/");

        let prefixed = BaseUrl::new("https://example.com/app")?;
        assert_eq!(
            prefixed.join("/assets/logo.png"),
            "https://example.com/app/assets/logo.png"
        );

        Ok(())
    }

    #[test]
    fn reads_the_registered_base_url_from_context() -> Result<(), BaseUrlError> {
        let cx = CxTestBuilder::new()
            .app_context(BaseUrl::new("https://example.com")?)
            .build();

        assert_eq!(base_url(&cx).as_str(), "https://example.com");
        assert_eq!(
            try_base_url(&cx),
            Some(&BaseUrl::new("https://example.com")?)
        );

        Ok(())
    }

    #[test]
    fn try_base_url_is_none_when_unregistered() {
        let cx = Cx::default();
        assert_eq!(try_base_url(&cx), None);
    }

    #[test]
    #[should_panic(expected = "attempted to access the base URL")]
    fn base_url_panics_when_unregistered() {
        let cx = Cx::default();
        let _ = base_url(&cx);
    }
}
