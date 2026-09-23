use http::{HeaderName, request::Parts};
use topcoat_core::context::{Cx, request_context};

use crate::header;

/// Reads the request header `name` as a string slice, or [`None`] when it is
/// absent or not valid UTF-8.
///
/// The value is borrowed from the request, so there is nothing worth caching
/// with `#[memoize]`.
#[track_caller]
fn header<'cx>(cx: &'cx Cx, name: &HeaderName) -> Option<&'cx str> {
    request_context::<Parts>(cx)
        .headers
        .get(name)?
        .to_str()
        .ok()
}

/// Returns `true` when the current request was sent by Alpine AJAX, which
/// means it carries an `X-Alpine-Request: true` header.
///
/// Use it to render only a fragment for Alpine AJAX requests and the full
/// page for normal browser requests.
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

/// Returns an iterator over the `id`s of the target elements, read from the
/// space-separated `X-Alpine-Target` header.
///
/// The iterator is empty when the header is absent.
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

/// Returns `true` when `id` is one of the target elements listed in the
/// `X-Alpine-Target` header.
///
/// Use it to skip rendering parts of the page that the client will not
/// merge.
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
