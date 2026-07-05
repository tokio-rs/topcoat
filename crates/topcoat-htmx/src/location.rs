use http::HeaderValue;
use http::response::Parts;
use serde::Serialize;
use serde_json::Value;
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_router::runtime::IntoResponseParts;

use crate::SwapOption;
use crate::header;

/// Performs a client-side redirect that does not trigger a full page reload,
/// via the `HX-Location` header.
///
/// In its simplest form it carries just a path. Set any of the [`LocationOptions`]
/// fields, through the builder methods, to control how htmx fetches and swaps
/// the new content; when any option is set, the header is serialized as JSON.
///
/// # Examples
///
/// ```rust
/// use topcoat::htmx::{HxLocation, SwapOption};
///
/// // Plain path: `HX-Location: /home`
/// let simple = HxLocation::new("/home");
///
/// // With options, serialized as JSON.
/// let detailed = HxLocation::new("/home")
///     .target("#main")
///     .swap(SwapOption::InnerHtml);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxLocation {
    /// The path (URL) to fetch the new content from.
    pub path: String,
    /// Optional context controlling the fetch and swap.
    pub options: LocationOptions,
}

impl HxLocation {
    /// Creates a location targeting `path` with no extra options.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            options: LocationOptions::default(),
        }
    }

    /// Sets the source element for the request (`source`).
    #[must_use]
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.options.source = Some(source.into());
        self
    }

    /// Sets the event that triggered the request (`event`).
    #[must_use]
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.options.event = Some(event.into());
        self
    }

    /// Sets a callback that handles the response (`handler`).
    #[must_use]
    pub fn handler(mut self, handler: impl Into<String>) -> Self {
        self.options.handler = Some(handler.into());
        self
    }

    /// Sets the element to swap the response into (`target`), a CSS selector.
    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.options.target = Some(target.into());
        self
    }

    /// Sets how the response is swapped in (`swap`).
    #[must_use]
    pub fn swap(mut self, swap: SwapOption) -> Self {
        self.options.swap = Some(swap);
        self
    }

    /// Sets a CSS selector choosing which part of the response is used (`select`).
    #[must_use]
    pub fn select(mut self, select: impl Into<String>) -> Self {
        self.options.select = Some(select.into());
        self
    }

    /// Sets values to submit with the request (`values`).
    #[must_use]
    pub fn values(mut self, values: Value) -> Self {
        self.options.values = Some(values);
        self
    }

    /// Sets headers to submit with the request (`headers`).
    #[must_use]
    pub fn headers(mut self, headers: Value) -> Self {
        self.options.headers = Some(headers);
        self
    }

    /// Renders the header value: the plain path when no options are set,
    /// otherwise a JSON object combining `path` with the options.
    fn header_value(self) -> Result<HeaderValue> {
        if self.options.is_empty() {
            return Ok(HeaderValue::from_str(&self.path)?);
        }

        let mut object = match serde_json::to_value(&self.options)? {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        object.insert("path".to_owned(), Value::String(self.path));
        Ok(HeaderValue::from_str(&serde_json::to_string(&object)?)?)
    }
}

impl From<&str> for HxLocation {
    fn from(path: &str) -> Self {
        Self::new(path)
    }
}

impl From<String> for HxLocation {
    fn from(path: String) -> Self {
        Self::new(path)
    }
}

impl IntoResponseParts for HxLocation {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts
            .headers
            .insert(header::HX_LOCATION, self.header_value()?);
        Ok(())
    }
}

/// The optional context of an [`HxLocation`].
///
/// Each field maps to a property of the htmx
/// [`HX-Location` JSON form](https://htmx.org/headers/hx-location/). Fields left
/// as [`None`] are omitted from the serialized header.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct LocationOptions {
    /// The source element of the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// An event that triggered the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// A callback that handles the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<String>,
    /// The target to swap the response into.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// How the response is swapped in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap: Option<SwapOption>,
    /// A CSS selector choosing which part of the response is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    /// Values to submit with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Value>,
    /// Headers to submit with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Value>,
}

impl LocationOptions {
    /// Returns `true` when no option is set, so the location can be sent as a
    /// plain path.
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_value(location: HxLocation) -> String {
        let mut parts = http::Response::new(()).into_parts().0;
        location
            .into_response_parts(&Cx::empty(), &mut parts)
            .unwrap();
        parts
            .headers
            .get(header::HX_LOCATION)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn plain_path_when_no_options() {
        assert_eq!(header_value(HxLocation::new("/home")), "/home");
    }

    #[test]
    fn options_serialize_as_json_with_path() {
        let value = header_value(
            HxLocation::new("/home")
                .target("#main")
                .swap(SwapOption::InnerHtml),
        );
        let json: Value = serde_json::from_str(&value).unwrap();
        assert_eq!(json["path"], "/home");
        assert_eq!(json["target"], "#main");
        assert_eq!(json["swap"], "innerHTML");
        // Unset options are omitted.
        assert!(json.get("source").is_none());
    }
}
