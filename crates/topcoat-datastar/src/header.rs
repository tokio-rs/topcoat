//! The Datastar HTTP header names, as [`HeaderName`] constants.
//!
//! See the [Datastar actions reference](https://data-star.dev/reference/actions)
//! for the full semantics of each header.

use http::HeaderName;

// -- Request headers (sent by Datastar to the server) --

/// `Datastar-Request`: always set on requests issued by a Datastar action.
pub const DATASTAR_REQUEST: HeaderName = HeaderName::from_static("datastar-request");

// -- Response headers (sent by the server to Datastar) --

/// `datastar-selector`: a CSS selector for the elements a `text/html` response
/// patches.
pub const DATASTAR_SELECTOR: HeaderName = HeaderName::from_static("datastar-selector");

/// `datastar-mode`: how the elements of a `text/html` response are patched
/// into the DOM.
pub const DATASTAR_MODE: HeaderName = HeaderName::from_static("datastar-mode");

/// `datastar-use-view-transition`: when `true`, a `text/html` response is
/// patched using the View Transition API.
pub const DATASTAR_USE_VIEW_TRANSITION: HeaderName =
    HeaderName::from_static("datastar-use-view-transition");

/// `datastar-only-if-missing`: when `true`, an `application/json` response
/// only patches signals that do not exist yet.
pub const DATASTAR_ONLY_IF_MISSING: HeaderName =
    HeaderName::from_static("datastar-only-if-missing");

/// `datastar-script-attributes`: a JSON object of attributes for the script
/// element a `text/javascript` response executes.
pub const DATASTAR_SCRIPT_ATTRIBUTES: HeaderName =
    HeaderName::from_static("datastar-script-attributes");
