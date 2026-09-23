//! The Alpine AJAX HTTP header names, as [`HeaderName`] constants.
//!
//! See the [Alpine AJAX reference](https://alpine-ajax.js.org/reference/) for
//! the full semantics of each header.

use http::HeaderName;

// -- Request headers (sent by Alpine AJAX to the server) --

/// `X-Alpine-Request`: always set to `true` on requests issued by Alpine AJAX.
pub const X_ALPINE_REQUEST: HeaderName = HeaderName::from_static("x-alpine-request");

/// `X-Alpine-Target`: a space-separated list of the `id`s of the target
/// elements being requested.
pub const X_ALPINE_TARGET: HeaderName = HeaderName::from_static("x-alpine-target");
