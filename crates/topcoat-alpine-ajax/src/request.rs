use http::{HeaderName, request::Parts};
use topcoat_core::context::{Cx, request_context};

use crate::header;

/// Borrows the header value, or returns [`None`] if it is missing or cannot
/// be represented as text.
#[track_caller]
fn header<'cx>(cx: &'cx Cx, name: &HeaderName) -> Option<&'cx str> {
    request_context::<Parts>(cx)
        .headers
        .get(name)?
        .to_str()
        .ok()
}

/// Returns whether the request carries `X-Alpine-Request: true`.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[inline]
#[must_use]
#[track_caller]
pub fn ajax_request(cx: &Cx) -> bool {
    header(cx, &header::X_ALPINE_REQUEST) == Some("true")
}

/// Returns the target element `id`s from the `X-Alpine-Target` header, or an
/// empty iterator when the header is absent.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[track_caller]
pub fn ajax_targets(cx: &Cx) -> impl Iterator<Item = &str> {
    header(cx, &header::X_ALPINE_TARGET)
        .unwrap_or_default()
        .split_whitespace()
}

/// Returns whether `id` appears in the `X-Alpine-Target` header.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in context).
#[must_use]
#[track_caller]
pub fn ajax_target(cx: &Cx, id: &str) -> bool {
    ajax_targets(cx).any(|target| target == id)
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    /// Builds a `Cx` whose request carries the given headers.
    fn cx_with(headers: &[(&HeaderName, &str)]) -> Cx {
        let mut builder = Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        CxTestBuilder::new().request_context(parts).build()
    }

    #[test]
    fn ajax_request_true_when_header_true() {
        let cx = cx_with(&[(&header::X_ALPINE_REQUEST, "true")]);
        assert!(ajax_request(&cx));
    }

    #[test]
    fn ajax_request_false_when_header_false() {
        let cx = cx_with(&[(&header::X_ALPINE_REQUEST, "false")]);
        assert!(!ajax_request(&cx));
    }

    #[test]
    fn ajax_request_false_when_header_missing() {
        let cx = cx_with(&[]);
        assert!(!ajax_request(&cx));
    }

    #[test]
    fn ajax_targets_empty_when_header_missing() {
        let cx = cx_with(&[]);
        assert_eq!(ajax_targets(&cx).count(), 0);
    }

    #[test]
    fn ajax_targets_empty_when_header_empty_string() {
        let cx = cx_with(&[(&header::X_ALPINE_TARGET, "")]);
        assert_eq!(ajax_targets(&cx).count(), 0);
    }

    #[test]
    fn ajax_targets_single_id() {
        let cx = cx_with(&[(&header::X_ALPINE_TARGET, "comments")]);
        assert_eq!(ajax_targets(&cx).collect::<Vec<_>>(), vec!["comments"]);
    }

    #[test]
    fn ajax_targets_multiple_space_separated_ids() {
        let cx = cx_with(&[(&header::X_ALPINE_TARGET, "comments comments_count")]);
        assert_eq!(
            ajax_targets(&cx).collect::<Vec<_>>(),
            vec!["comments", "comments_count"]
        );
    }

    #[test]
    fn ajax_targets_tolerates_extra_whitespace() {
        let cx = cx_with(&[(&header::X_ALPINE_TARGET, "  comments   count  ")]);
        assert_eq!(
            ajax_targets(&cx).collect::<Vec<_>>(),
            vec!["comments", "count"]
        );
    }

    #[test]
    fn ajax_target_matches_one_of_multiple_ids() {
        let cx = cx_with(&[(&header::X_ALPINE_TARGET, "comments comments_count")]);
        assert!(ajax_target(&cx, "comments_count"));
        assert!(!ajax_target(&cx, "sidebar"));
    }

    #[test]
    fn ajax_target_false_when_header_missing() {
        let cx = cx_with(&[]);
        assert!(!ajax_target(&cx, "comments"));
    }
}
