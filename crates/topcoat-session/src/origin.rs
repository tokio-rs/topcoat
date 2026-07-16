use topcoat_core::context::{Cx, CxBuilder};
use topcoat_router::{
    Body, ForbiddenError, Layer, LayerFuture, Method, Next, Path, forbidden, header, headers,
    method, uri,
};

use crate::config;

/// Verifies that the current request is not a cross-site request forgery
/// (CSRF).
///
/// Requests with safe methods (`GET`, `HEAD`, `OPTIONS`) always pass. For
/// every other method, the browser-provided `Sec-Fetch-Site` header must be
/// `same-origin` or `none` (a direct navigation); `same-site` is rejected, so
/// sibling subdomains cannot forge requests. For browsers that predate
/// `Sec-Fetch-Site`, the `Origin` header's host is compared against the
/// request's own host instead. Requests carrying neither header pass: they
/// come from non-browser clients, which do not attach cookies ambiently.
///
/// Origins trusted with [`Config::trust_origin`](crate::Config::trust_origin)
/// always pass. The [`OriginLayer`] registered by the router's `sessions`
/// extension method applies this check to every request; call it directly
/// only where that layer does not.
///
/// # Errors
///
/// Returns a [`ForbiddenError`] (HTTP 403) when the request is a
/// state-changing cross-origin request.
pub fn verify_origin(cx: &Cx) -> Result<(), ForbiddenError> {
    let headers = headers(cx);
    let sec_fetch_site = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok());
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| uri(cx).authority().map(|authority| authority.as_str()));

    if allowed(
        method(cx),
        sec_fetch_site,
        origin,
        host,
        &config(cx).trusted_origins,
    ) {
        Ok(())
    } else {
        Err(forbidden())
    }
}

/// The decision behind [`verify_origin`], over the request's relevant parts.
fn allowed(
    method: &Method,
    sec_fetch_site: Option<&str>,
    origin: Option<&str>,
    host: Option<&str>,
    trusted_origins: &[String],
) -> bool {
    // Safe methods must not change state, so they need no protection.
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return true;
    }
    // An explicitly trusted origin passes regardless of how the browser
    // classified the request.
    if origin.is_some_and(|origin| {
        trusted_origins
            .iter()
            .any(|trusted| trusted.eq_ignore_ascii_case(origin))
    }) {
        return true;
    }
    // A modern browser declares how the request's initiator relates to its
    // target.
    if let Some(site) = sec_fetch_site {
        return matches!(site, "same-origin" | "none");
    }
    // Older browsers send only `Origin`; compare its host against the
    // request's own. `Origin: null` has no host and never matches.
    if let Some(origin) = origin {
        return origin_host(origin)
            .zip(host)
            .is_some_and(|(origin_host, host)| origin_host.eq_ignore_ascii_case(host));
    }
    // Neither header: not a browser, so no ambient cookies to forge with.
    true
}

/// Extracts the `host[:port]` part of a serialized origin.
fn origin_host(origin: &str) -> Option<&str> {
    origin.split_once("://").map(|(_, host)| host)
}

/// A router layer that rejects state-changing cross-origin requests, as a
/// defense against cross-site request forgery (CSRF).
///
/// Every request passing through it is checked with [`verify_origin`].
/// The router's `sessions` extension method registers it unless
/// [`Config::dangerous_disable_origin_verification`](crate::Config::dangerous_disable_origin_verification)
/// is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct OriginLayer;

impl OriginLayer {
    /// Creates an origin verification layer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Layer for OriginLayer {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        match verify_origin(cx) {
            Ok(()) => next.run(cx, body),
            Err(error) => Box::pin(async move { Err(error.into()) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_TRUST: &[String] = &[];

    fn post_allowed(
        sec_fetch_site: Option<&str>,
        origin: Option<&str>,
        host: Option<&str>,
    ) -> bool {
        allowed(&Method::POST, sec_fetch_site, origin, host, NO_TRUST)
    }

    #[test]
    fn safe_methods_pass_even_cross_site() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert!(allowed(
                &method,
                Some("cross-site"),
                Some("https://evil.example"),
                Some("app.example"),
                NO_TRUST,
            ));
        }
    }

    #[test]
    fn same_origin_and_direct_navigation_pass() {
        assert!(post_allowed(Some("same-origin"), None, Some("app.example")));
        assert!(post_allowed(Some("none"), None, Some("app.example")));
    }

    #[test]
    fn same_site_and_cross_site_are_rejected() {
        // `same-site` is rejected deliberately: a sibling subdomain must not
        // be able to forge requests.
        assert!(!post_allowed(
            Some("same-site"),
            Some("https://evil.app.example"),
            Some("app.example"),
        ));
        assert!(!post_allowed(
            Some("cross-site"),
            Some("https://evil.example"),
            Some("app.example"),
        ));
    }

    #[test]
    fn trusted_origins_pass_even_cross_site() {
        let trusted = vec!["https://accounts.example".to_owned()];
        assert!(allowed(
            &Method::POST,
            Some("cross-site"),
            Some("https://accounts.example"),
            Some("app.example"),
            &trusted,
        ));
        // Trust is keyed on the `Origin` header; without one the request is
        // still classified by `Sec-Fetch-Site`.
        assert!(!allowed(
            &Method::POST,
            Some("cross-site"),
            None,
            Some("app.example"),
            &trusted,
        ));
    }

    #[test]
    fn origin_fallback_compares_hosts() {
        assert!(post_allowed(
            None,
            Some("https://app.example"),
            Some("app.example"),
        ));
        // Hosts are case-insensitive, and any explicit port must match.
        assert!(post_allowed(
            None,
            Some("https://App.Example:8443"),
            Some("app.example:8443"),
        ));
        assert!(!post_allowed(
            None,
            Some("https://evil.example"),
            Some("app.example"),
        ));
        assert!(!post_allowed(
            None,
            Some("https://app.example:8443"),
            Some("app.example"),
        ));
    }

    #[test]
    fn opaque_origin_is_rejected() {
        assert!(!post_allowed(None, Some("null"), Some("app.example")));
    }

    #[test]
    fn origin_without_a_request_host_is_rejected() {
        assert!(!post_allowed(None, Some("https://app.example"), None));
    }

    #[test]
    fn non_browser_requests_pass() {
        assert!(post_allowed(None, None, Some("app.example")));
        assert!(post_allowed(None, None, None));
    }
}
