use http::{HeaderName, request::Parts};
use topcoat_core::runtime::context::{Cx, request_context};

use crate::header;

/// Reads the request header `name` as a string slice, or [`None`] when it is
/// absent or not valid UTF-8.
///
/// The header map is borrowed straight from the request, so these reads are
/// cheap pointer lookups: there is nothing worth caching with `#[memoize]`,
/// and borrowing avoids the allocation a memoized owned value would require.
fn header<'cx>(cx: &'cx Cx, name: &HeaderName) -> Option<&'cx str> {
    request_context::<Parts>(cx)
        .headers
        .get(name)?
        .to_str()
        .ok()
}

/// Returns `true` when the current request was issued by htmx, i.e. it carries
/// an `HX-Request: true` header.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_request(cx: &Cx) -> bool {
    header(cx, &header::HX_REQUEST) == Some("true")
}

/// Returns `true` when the request was made by an element using `hx-boost`,
/// i.e. it carries `HX-Boosted: true`.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_boosted(cx: &Cx) -> bool {
    header(cx, &header::HX_BOOSTED) == Some("true")
}

/// Returns `true` when the request restores history after a miss in the local
/// history cache, i.e. it carries `HX-History-Restore-Request: true`.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_history_restore_request(cx: &Cx) -> bool {
    header(cx, &header::HX_HISTORY_RESTORE_REQUEST) == Some("true")
}

/// Returns the current browser URL from the `HX-Current-URL` header, or [`None`]
/// when it is absent.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_current_url(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_CURRENT_URL)
}

/// Returns the user's response to an `hx-prompt` from the `HX-Prompt` header,
/// or [`None`] when there was no prompt.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_prompt(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_PROMPT)
}

/// Returns the `id` of the target element from the `HX-Target` header, or
/// [`None`] when the request has no target.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_target(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TARGET)
}

/// Returns the `id` of the triggering element from the `HX-Trigger` header, or
/// [`None`] when it has none.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_trigger(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TRIGGER)
}

/// Returns the `name` of the triggering element from the `HX-Trigger-Name`
/// header, or [`None`] when it has none.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
pub fn hx_trigger_name(cx: &Cx) -> Option<&str> {
    header(cx, &header::HX_TRIGGER_NAME)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::Request;
    use topcoat_core::runtime::context::ContextMap;

    use super::*;

    /// Builds a `Cx` whose request carries the given headers.
    fn cx_with(headers: &[(&HeaderName, &str)]) -> Cx {
        let mut builder = Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        let mut request_context = ContextMap::new();
        request_context.insert::<Parts>(parts);
        Cx::new(Arc::new(ContextMap::new()), request_context)
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
}
