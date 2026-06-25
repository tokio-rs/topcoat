use http::HeaderValue;
use http::response::Parts;
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_router::runtime::IntoResponseParts;

use crate::SwapOption;
use crate::header;

/// Pushes a new URL onto the browser history stack via the `HX-Push-Url`
/// header.
///
/// Construct it from a URL string. Use [`HxPushUrl::prevent`] to send
/// `HX-Push-Url: false`, which stops htmx from updating history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxPushUrl(pub String);

impl HxPushUrl {
    /// Sends `HX-Push-Url: false`, preventing htmx from updating the history.
    #[must_use]
    pub fn prevent() -> Self {
        Self("false".to_owned())
    }
}

impl<T: Into<String>> From<T> for HxPushUrl {
    fn from(url: T) -> Self {
        Self(url.into())
    }
}

impl IntoResponseParts for HxPushUrl {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_PUSH_URL, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

/// Replaces the current URL in the location bar via the `HX-Replace-Url`
/// header.
///
/// Use [`HxReplaceUrl::prevent`] to send `HX-Replace-Url: false`, which stops
/// htmx from updating the location bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxReplaceUrl(pub String);

impl HxReplaceUrl {
    /// Sends `HX-Replace-Url: false`, preventing htmx from updating the URL.
    #[must_use]
    pub fn prevent() -> Self {
        Self("false".to_owned())
    }
}

impl<T: Into<String>> From<T> for HxReplaceUrl {
    fn from(url: T) -> Self {
        Self(url.into())
    }
}

impl IntoResponseParts for HxReplaceUrl {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_REPLACE_URL, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

/// Performs a client-side redirect to a new location via the `HX-Redirect`
/// header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxRedirect(pub String);

impl<T: Into<String>> From<T> for HxRedirect {
    fn from(url: T) -> Self {
        Self(url.into())
    }
}

impl IntoResponseParts for HxRedirect {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_REDIRECT, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

/// Triggers a full client-side page refresh via the `HX-Refresh` header when
/// `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HxRefresh(pub bool);

impl From<bool> for HxRefresh {
    fn from(refresh: bool) -> Self {
        Self(refresh)
    }
}

impl IntoResponseParts for HxRefresh {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        let value = if self.0 { "true" } else { "false" };
        parts
            .headers
            .insert(header::HX_REFRESH, HeaderValue::from_static(value));
        Ok(())
    }
}

/// Overrides how the response is swapped in via the `HX-Reswap` header. See
/// [`SwapOption`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HxReswap(pub SwapOption);

impl From<SwapOption> for HxReswap {
    fn from(option: SwapOption) -> Self {
        Self(option)
    }
}

impl IntoResponseParts for HxReswap {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts.headers.insert(header::HX_RESWAP, self.0.into());
        Ok(())
    }
}

/// Retargets the content update to a different element via the `HX-Retarget`
/// header. The value is a CSS selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxRetarget(pub String);

impl<T: Into<String>> From<T> for HxRetarget {
    fn from(selector: T) -> Self {
        Self(selector.into())
    }
}

impl IntoResponseParts for HxRetarget {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_RETARGET, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

/// Chooses which part of the response is swapped in via the `HX-Reselect`
/// header, overriding an existing `hx-select`. The value is a CSS selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxReselect(pub String);

impl<T: Into<String>> From<T> for HxReselect {
    fn from(selector: T) -> Self {
        Self(selector.into())
    }
}

impl IntoResponseParts for HxReselect {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_RESELECT, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts() -> Parts {
        http::Response::new(()).into_parts().0
    }

    fn header_value(parts: &Parts, name: &http::HeaderName) -> String {
        parts.headers.get(name).unwrap().to_str().unwrap().to_owned()
    }

    #[test]
    fn push_url_sets_header() {
        let mut parts = parts();
        HxPushUrl::from("/new").into_response_parts(&Cx::empty(), &mut parts).unwrap();
        assert_eq!(header_value(&parts, &header::HX_PUSH_URL), "/new");
    }

    #[test]
    fn push_url_prevent_is_false() {
        let mut parts = parts();
        HxPushUrl::prevent().into_response_parts(&Cx::empty(), &mut parts).unwrap();
        assert_eq!(header_value(&parts, &header::HX_PUSH_URL), "false");
    }

    #[test]
    fn refresh_serializes_bool() {
        let mut parts = parts();
        HxRefresh(true).into_response_parts(&Cx::empty(), &mut parts).unwrap();
        assert_eq!(header_value(&parts, &header::HX_REFRESH), "true");
    }

    #[test]
    fn reswap_uses_swap_option_string() {
        let mut parts = parts();
        HxReswap(SwapOption::BeforeEnd)
            .into_response_parts(&Cx::empty(), &mut parts)
            .unwrap();
        assert_eq!(header_value(&parts, &header::HX_RESWAP), "beforeend");
    }

    #[test]
    fn retarget_and_reselect_carry_selectors() {
        let mut parts = parts();
        HxRetarget::from("#main")
            .into_response_parts(&Cx::empty(), &mut parts)
            .unwrap();
        HxReselect::from(".item")
            .into_response_parts(&Cx::empty(), &mut parts)
            .unwrap();
        assert_eq!(header_value(&parts, &header::HX_RETARGET), "#main");
        assert_eq!(header_value(&parts, &header::HX_RESELECT), ".item");
    }
}
