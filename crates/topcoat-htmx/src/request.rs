use http::{HeaderName, request::Parts};
use topcoat_core::context::{Cx, try_context};

use crate::header;

/// Borrows the header value, or returns [`None`] if it is missing, cannot be
/// represented as text, or `cx` does not belong to a router request.
fn header<'cx>(cx: &'cx Cx, name: &HeaderName) -> Option<&'cx str> {
    try_context::<Parts>(cx)?.headers.get(name)?.to_str().ok()
}

/// Returns whether the request carries `HX-Request: true`.
#[inline]
#[must_use]
pub fn hx_request(cx: &Cx) -> bool {
    header(cx, &header::HX_REQUEST) == Some("true")
}

/// Returns whether `HX-Boosted` is `true`, indicating an `hx-boost` request.
#[inline]
#[must_use]
pub fn hx_boosted(cx: &Cx) -> bool {
    header(cx, &header::HX_BOOSTED) == Some("true")
}

/// Returns whether `HX-History-Restore-Request` is `true`, indicating that htmx
/// is restoring a page missing from its history cache.
#[inline]
#[must_use]
pub fn hx_history_restore_request(cx: &Cx) -> bool {
    header(cx, &header::HX_HISTORY_RESTORE_REQUEST) == Some("true")
}

/// Returns the current browser URL from the `HX-Current-URL` header, or [`None`]
/// when it is absent.
#[inline]
#[must_use]
pub fn hx_current_url(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_CURRENT_URL)
}

/// Returns the user's response to an `hx-prompt` from the `HX-Prompt` header,
/// or [`None`] when there was no prompt.
#[inline]
#[must_use]
pub fn hx_prompt(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_PROMPT)
}

/// Returns the `id` of the target element from the `HX-Target` header, or
/// [`None`] when the request has no target.
#[inline]
#[must_use]
pub fn hx_target(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TARGET)
}

/// Returns the `id` of the triggering element from the `HX-Trigger` header, or
/// [`None`] when it has none.
#[inline]
#[must_use]
pub fn hx_trigger(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TRIGGER)
}

/// Returns the `name` of the triggering element from the `HX-Trigger-Name`
/// header, or [`None`] when it has none.
#[inline]
#[must_use]
pub fn hx_trigger_name(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TRIGGER_NAME)
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::Cx;

    use super::*;

    /// Builds a `Cx` whose request carries the given headers.
    fn cx_with(headers: &[(&HeaderName, &str)]) -> Cx {
        let mut builder = Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        Cx::default().with(parts)
    }

    #[test]
    fn boolean_headers_require_true() {
        let cx = cx_with(&[
            (&header::HX_REQUEST, "true"),
            (&header::HX_BOOSTED, "false"),
        ]);
        assert!(hx_request(&cx));
        assert!(!hx_boosted(&cx));
        assert!(!hx_history_restore_request(&cx));
    }

    #[test]
    fn missing_boolean_header_is_false() {
        let cx = cx_with(&[]);
        assert!(!hx_request(&cx));
    }

    #[test]
    fn string_headers_are_borrowed() {
        let cx = cx_with(&[
            (&header::HX_CURRENT_URL, "https://example.com/page"),
            (&header::HX_PROMPT, "Ada"),
            (&header::HX_TARGET, "main"),
            (&header::HX_TRIGGER, "save-btn"),
            (&header::HX_TRIGGER_NAME, "save"),
        ]);
        assert_eq!(hx_current_url(&cx), Some("https://example.com/page"));
        assert_eq!(hx_prompt(&cx), Some("Ada"));
        assert_eq!(hx_target(&cx), Some("main"));
        assert_eq!(hx_trigger(&cx), Some("save-btn"));
        assert_eq!(hx_trigger_name(&cx), Some("save"));
    }

    #[test]
    fn missing_string_header_is_none() {
        let cx = cx_with(&[]);
        assert_eq!(hx_current_url(&cx), None);
        assert_eq!(hx_prompt(&cx), None);
    }

    #[test]
    fn headers_are_absent_outside_a_request() {
        let cx = Cx::default();
        assert!(!hx_request(&cx));
        assert_eq!(hx_target(&cx), None);
    }
}
