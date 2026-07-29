//! The Inertia.js v3 HTTP header names, as [`HeaderName`] constants.
//!
//! See the [Inertia.js protocol](https://inertiajs.com/docs/v3/core-concepts/the-protocol)
//! for the complete page and visit contract.

use http::HeaderName;

// -- Request headers (sent by an Inertia client) --

/// `X-Inertia`: its presence identifies an Inertia request.
pub const X_INERTIA: HeaderName = HeaderName::from_static("x-inertia");

/// `X-Inertia-Version`: the asset version held by the client.
pub const X_INERTIA_VERSION: HeaderName = HeaderName::from_static("x-inertia-version");

/// `X-Inertia-Partial-Component`: the component targeted by a partial reload.
pub const X_INERTIA_PARTIAL_COMPONENT: HeaderName =
    HeaderName::from_static("x-inertia-partial-component");

/// `X-Inertia-Partial-Data`: comma-separated prop paths to include.
pub const X_INERTIA_PARTIAL_DATA: HeaderName = HeaderName::from_static("x-inertia-partial-data");

/// `X-Inertia-Partial-Except`: comma-separated prop paths to exclude.
pub const X_INERTIA_PARTIAL_EXCEPT: HeaderName =
    HeaderName::from_static("x-inertia-partial-except");

/// `X-Inertia-Reset`: prop paths whose client-side merge state must reset.
pub const X_INERTIA_RESET: HeaderName = HeaderName::from_static("x-inertia-reset");

/// `X-Inertia-Error-Bag`: the validation error bag selected by the client.
pub const X_INERTIA_ERROR_BAG: HeaderName = HeaderName::from_static("x-inertia-error-bag");

/// `X-Inertia-Infinite-Scroll-Merge-Intent`: the requested scroll merge direction.
pub const X_INERTIA_INFINITE_SCROLL_MERGE_INTENT: HeaderName =
    HeaderName::from_static("x-inertia-infinite-scroll-merge-intent");

/// `X-Inertia-Except-Once-Props`: once-prop keys already held by the client.
pub const X_INERTIA_EXCEPT_ONCE_PROPS: HeaderName =
    HeaderName::from_static("x-inertia-except-once-props");

/// `Purpose`: `prefetch` identifies a speculative Inertia visit.
pub const PURPOSE: HeaderName = HeaderName::from_static("purpose");

// -- Response headers (sent by the server) --

/// `X-Inertia-Location`: asks the client to perform a full location visit.
pub const X_INERTIA_LOCATION: HeaderName = HeaderName::from_static("x-inertia-location");

/// `X-Inertia-Redirect`: asks the v3 client to follow a fragment redirect.
pub const X_INERTIA_REDIRECT: HeaderName = HeaderName::from_static("x-inertia-redirect");
