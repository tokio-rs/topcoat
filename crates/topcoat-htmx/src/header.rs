//! The htmx HTTP header names, as [`HeaderName`] constants.
//!
//! See the [htmx header reference](https://htmx.org/reference/#headers) for the
//! full semantics of each header.

use http::HeaderName;

// -- Request headers (sent by htmx to the server) --

/// `HX-Boosted`: set when the request comes from an element using `hx-boost`.
pub const HX_BOOSTED: HeaderName = HeaderName::from_static("hx-boosted");

/// `HX-Current-URL`: the current URL of the browser.
pub const HX_CURRENT_URL: HeaderName = HeaderName::from_static("hx-current-url");

/// `HX-History-Restore-Request`: set when the request restores history after a
/// miss in the local history cache.
pub const HX_HISTORY_RESTORE_REQUEST: HeaderName =
    HeaderName::from_static("hx-history-restore-request");

/// `HX-Prompt`: the user's response to an `hx-prompt`.
pub const HX_PROMPT: HeaderName = HeaderName::from_static("hx-prompt");

/// `HX-Request`: always set on requests issued by htmx.
pub const HX_REQUEST: HeaderName = HeaderName::from_static("hx-request");

/// `HX-Target`: the `id` of the target element, if any.
pub const HX_TARGET: HeaderName = HeaderName::from_static("hx-target");

/// `HX-Trigger-Name`: the `name` of the triggering element, if any.
pub const HX_TRIGGER_NAME: HeaderName = HeaderName::from_static("hx-trigger-name");

/// `HX-Trigger`: the `id` of the triggering element, if any (on requests).
///
/// On responses, this header instead carries client-side events to trigger.
pub const HX_TRIGGER: HeaderName = HeaderName::from_static("hx-trigger");

// -- Response headers (sent by the server to htmx) --

/// `HX-Location`: performs a client-side redirect without a full page reload.
pub const HX_LOCATION: HeaderName = HeaderName::from_static("hx-location");

/// `HX-Push-Url`: pushes a new URL onto the browser history stack.
pub const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");

/// `HX-Redirect`: performs a client-side redirect to a new location.
pub const HX_REDIRECT: HeaderName = HeaderName::from_static("hx-redirect");

/// `HX-Refresh`: when `true`, the client does a full page refresh.
pub const HX_REFRESH: HeaderName = HeaderName::from_static("hx-refresh");

/// `HX-Replace-Url`: replaces the current URL in the browser location bar.
pub const HX_REPLACE_URL: HeaderName = HeaderName::from_static("hx-replace-url");

/// `HX-Reswap`: specifies how the response is swapped in. See `hx-swap`.
pub const HX_RESWAP: HeaderName = HeaderName::from_static("hx-reswap");

/// `HX-Retarget`: a CSS selector that retargets the content update.
pub const HX_RETARGET: HeaderName = HeaderName::from_static("hx-retarget");

/// `HX-Reselect`: a CSS selector choosing which part of the response is
/// swapped in, overriding an existing `hx-select`.
pub const HX_RESELECT: HeaderName = HeaderName::from_static("hx-reselect");

/// `HX-Trigger-After-Settle`: triggers client-side events after the settle
/// step.
pub const HX_TRIGGER_AFTER_SETTLE: HeaderName = HeaderName::from_static("hx-trigger-after-settle");

/// `HX-Trigger-After-Swap`: triggers client-side events after the swap step.
pub const HX_TRIGGER_AFTER_SWAP: HeaderName = HeaderName::from_static("hx-trigger-after-swap");
