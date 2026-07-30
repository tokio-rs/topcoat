use http::HeaderValue;
use http::response::Parts;
use serde::Serialize;
use serde_json::Value;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::IntoResponseParts;

use crate::{ElementPatchMode, common, header};

/// Targets the elements a `text/html` response patches via the
/// `datastar-selector` header. The value is a CSS selector.
///
/// Without it, Datastar matches the response's elements to the DOM by their
/// `id` attribute.
///
/// # Panics
///
/// Converting from a selector containing a line break panics. Applying a
/// directly constructed value containing a line break also panics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatastarSelector(pub String);

impl<T: Into<String>> From<T> for DatastarSelector {
    #[track_caller]
    fn from(selector: T) -> Self {
        let selector = selector.into();
        common::assert_valid_selector(&selector);
        Self(selector)
    }
}

impl IntoResponseParts for DatastarSelector {
    #[track_caller]
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        common::assert_valid_selector(&self.0);
        parts
            .headers
            .insert(header::DATASTAR_SELECTOR, HeaderValue::from_str(&self.0)?);
        Ok(())
    }
}

/// Sets how a `text/html` response's elements are patched into the DOM via
/// the `datastar-mode` header. See [`ElementPatchMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatastarMode(pub ElementPatchMode);

impl From<ElementPatchMode> for DatastarMode {
    fn from(mode: ElementPatchMode) -> Self {
        Self(mode)
    }
}

impl IntoResponseParts for DatastarMode {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts.headers.insert(header::DATASTAR_MODE, self.0.into());
        Ok(())
    }
}

/// Patches a `text/html` response using the View Transition API via the
/// `datastar-use-view-transition` header when `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatastarUseViewTransition(pub bool);

impl From<bool> for DatastarUseViewTransition {
    fn from(use_view_transition: bool) -> Self {
        Self(use_view_transition)
    }
}

impl IntoResponseParts for DatastarUseViewTransition {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        let value = if self.0 { "true" } else { "false" };
        parts.headers.insert(
            header::DATASTAR_USE_VIEW_TRANSITION,
            HeaderValue::from_static(value),
        );
        Ok(())
    }
}

/// Restricts an `application/json` response to patching signals that do not
/// exist yet, via the `datastar-only-if-missing` header when `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatastarOnlyIfMissing(pub bool);

impl From<bool> for DatastarOnlyIfMissing {
    fn from(only_if_missing: bool) -> Self {
        Self(only_if_missing)
    }
}

impl IntoResponseParts for DatastarOnlyIfMissing {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        let value = if self.0 { "true" } else { "false" };
        parts.headers.insert(
            header::DATASTAR_ONLY_IF_MISSING,
            HeaderValue::from_static(value),
        );
        Ok(())
    }
}

/// Sets the attributes of the script element a `text/javascript` response
/// executes, via the `datastar-script-attributes` header.
///
/// The value is a JSON object mapping attribute names to values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatastarScriptAttributes(pub Value);

impl DatastarScriptAttributes {
    /// Serializes `attributes` into the JSON object the header carries.
    ///
    /// # Errors
    ///
    /// Returns an error when `attributes` cannot be serialized.
    pub fn new(attributes: impl Serialize) -> Result<Self> {
        Ok(Self(serde_json::to_value(attributes)?))
    }
}

impl From<Value> for DatastarScriptAttributes {
    fn from(attributes: Value) -> Self {
        Self(attributes)
    }
}

impl IntoResponseParts for DatastarScriptAttributes {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts.headers.insert(
            header::DATASTAR_SCRIPT_ATTRIBUTES,
            HeaderValue::from_str(&serde_json::to_string(&self.0)?)?,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parts() -> Parts {
        http::Response::new(()).into_parts().0
    }

    fn header_value(parts: &Parts, name: &http::HeaderName) -> String {
        parts
            .headers
            .get(name)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn selector_carries_the_css_selector() {
        let mut parts = parts();
        DatastarSelector::from("#feed")
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        assert_eq!(header_value(&parts, &header::DATASTAR_SELECTOR), "#feed");
    }

    #[test]
    fn selector_line_breaks_panic() {
        for selector in [
            "#feed\nelements <img src=x onerror=alert(1)>",
            "#feed\relements <img src=x onerror=alert(1)>",
            "#feed\r\nelements <img src=x onerror=alert(1)>",
        ] {
            assert!(
                std::panic::catch_unwind(|| DatastarSelector::from(selector)).is_err(),
                "conversion should panic for {selector:?}"
            );

            let mut parts = parts();
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    DatastarSelector(selector.to_owned())
                        .into_response_parts(&Cx::default(), &mut parts)
                        .unwrap();
                }))
                .is_err(),
                "direct construction should panic for {selector:?}"
            );
        }
    }

    #[test]
    fn mode_uses_the_patch_mode_string() {
        let mut parts = parts();
        DatastarMode(ElementPatchMode::Append)
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        assert_eq!(header_value(&parts, &header::DATASTAR_MODE), "append");
    }

    #[test]
    fn booleans_serialize_as_true_and_false() {
        let mut parts = parts();
        DatastarUseViewTransition(true)
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        DatastarOnlyIfMissing(false)
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        assert_eq!(
            header_value(&parts, &header::DATASTAR_USE_VIEW_TRANSITION),
            "true"
        );
        assert_eq!(
            header_value(&parts, &header::DATASTAR_ONLY_IF_MISSING),
            "false"
        );
    }

    #[test]
    fn script_attributes_serialize_as_json() {
        let mut parts = parts();
        DatastarScriptAttributes::new(json!({ "type": "module" }))
            .unwrap()
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        assert_eq!(
            header_value(&parts, &header::DATASTAR_SCRIPT_ATTRIBUTES),
            r#"{"type":"module"}"#
        );
    }
}
