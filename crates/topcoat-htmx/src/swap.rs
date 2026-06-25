use http::HeaderValue;
use serde::Serialize;

/// How htmx swaps a response into the DOM.
///
/// Mirrors the values accepted by the `hx-swap` attribute. Used by
/// [`HxReswap`](crate::HxReswap) and as the `swap` field of an
/// [`HxLocation`](crate::HxLocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SwapOption {
    /// Replace the inner HTML of the target element.
    #[serde(rename = "innerHTML")]
    InnerHtml,
    /// Replace the entire target element.
    #[serde(rename = "outerHTML")]
    OuterHtml,
    /// Insert the response before the target element.
    #[serde(rename = "beforebegin")]
    BeforeBegin,
    /// Insert the response before the first child of the target element.
    #[serde(rename = "afterbegin")]
    AfterBegin,
    /// Insert the response after the last child of the target element.
    #[serde(rename = "beforeend")]
    BeforeEnd,
    /// Insert the response after the target element.
    #[serde(rename = "afterend")]
    AfterEnd,
    /// Delete the target element regardless of the response.
    #[serde(rename = "delete")]
    Delete,
    /// Do not append the response to the target element.
    #[serde(rename = "none")]
    None,
}

impl SwapOption {
    /// Returns the htmx string for this swap option (e.g. `"innerHTML"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InnerHtml => "innerHTML",
            Self::OuterHtml => "outerHTML",
            Self::BeforeBegin => "beforebegin",
            Self::AfterBegin => "afterbegin",
            Self::BeforeEnd => "beforeend",
            Self::AfterEnd => "afterend",
            Self::Delete => "delete",
            Self::None => "none",
        }
    }
}

impl From<SwapOption> for HeaderValue {
    fn from(option: SwapOption) -> Self {
        HeaderValue::from_static(option.as_str())
    }
}
