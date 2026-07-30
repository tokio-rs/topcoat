use topcoat_core::context::Cx;

use crate::{
    Method, header,
    request::{headers, method, uri},
};

/// The cross-origin request policy the router applies to every request.
///
/// Browsers attach cookies to cross-origin requests, and an intranet or
/// localhost server is reachable by any page open in the browser, so a
/// malicious page can send requests the application cannot tell apart from
/// its own. By default the router rejects state-changing cross-origin browser
/// requests (cross-site request forgery) and cross-origin WebSocket
/// handshakes (cross-site WebSocket hijacking) unless the application
/// explicitly trusts the origin.
///
/// Same-origin requests, direct navigations, and requests from non-browser
/// clients (which carry no ambient credentials) always pass. Register a
/// policy with [`RouterBuilder::origin_policy`](crate::RouterBuilder::origin_policy)
/// to trust cross-origin peers or to opt out of verification.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{OriginPolicy, Router};
///
/// let router = Router::builder()
///     .origin_policy(OriginPolicy::trust_origins([
///         "https://accounts.example.com",
///     ]))
///     .build();
/// ```
#[derive(Debug)]
pub struct OriginPolicy {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    /// Verifies every request, trusting the listed cross-origin peers.
    Verify { trusted_origins: Vec<String> },
    /// No verification at all.
    Disabled,
}

impl OriginPolicy {
    /// Verifies origins, trusting `origins` to send state-changing
    /// cross-origin requests and to open cross-origin WebSockets.
    ///
    /// Each value is compared against the request's `Origin` header, so pass
    /// the full serialized origin: scheme, host, and any non-default port
    /// (`"https://accounts.example.com"`), with no trailing slash. The
    /// comparison is ASCII case-insensitive.
    pub fn trust_origins<I>(origins: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            inner: Inner::Verify {
                trusted_origins: origins.into_iter().map(Into::into).collect(),
            },
        }
    }

    /// Turns origin verification off entirely.
    ///
    /// Without it, nothing rejects state-changing cross-origin requests or
    /// cross-origin WebSocket handshakes; only use this if the application
    /// enforces its own defense against cross-site request forgery.
    #[must_use]
    pub fn dangerous_disable() -> Self {
        Self {
            inner: Inner::Disabled,
        }
    }

    /// Classifies the current request under this policy.
    pub(crate) fn check(&self, cx: &Cx) -> OriginVerdict {
        let Inner::Verify { trusted_origins } = &self.inner else {
            return OriginVerdict::Allow;
        };

        let headers = headers(cx);
        let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
        let origin = header(header::ORIGIN.as_str());

        // A WebSocket handshake is a GET, but must not be treated as safe:
        // the danger is not a state change during the handshake but the
        // cross-origin channel it opens. Modern browsers declare a handshake
        // with `Sec-Fetch-Mode: websocket`; the `Upgrade` header (a token
        // list) covers everything older.
        let websocket = method(cx) == Method::GET
            && (header("sec-fetch-mode") == Some("websocket")
                || header(header::UPGRADE.as_str()).is_some_and(|upgrade| {
                    upgrade
                        .split(',')
                        .any(|token| token.trim().eq_ignore_ascii_case("websocket"))
                }));

        // Safe methods must not change state, so they need no protection.
        if !websocket && matches!(*method(cx), Method::GET | Method::HEAD | Method::OPTIONS) {
            return OriginVerdict::Allow;
        }

        // An explicitly trusted origin passes regardless of how the browser
        // classified the request.
        if origin.is_some_and(|origin| {
            trusted_origins
                .iter()
                .any(|trusted| trusted.eq_ignore_ascii_case(origin))
        }) {
            return OriginVerdict::Allow;
        }

        let cross_origin = 'verdict: {
            // A modern browser declares how the request's initiator relates
            // to its target. `same-site` counts as cross-origin deliberately:
            // a sibling subdomain must not be able to forge requests.
            if let Some(site) = header("sec-fetch-site") {
                break 'verdict !matches!(site, "same-origin" | "none");
            }
            // Older browsers send only `Origin`; compare its host against the
            // request's own. `Origin: null` has no host and never matches.
            if let Some(origin) = origin {
                let origin_host = origin.split_once("://").map(|(_, host)| host);
                let host = header(header::HOST.as_str())
                    .or_else(|| uri(cx).authority().map(http::uri::Authority::as_str));
                break 'verdict !origin_host
                    .zip(host)
                    .is_some_and(|(origin_host, host)| origin_host.eq_ignore_ascii_case(host));
            }
            // Neither header: not a browser, so no ambient credentials to
            // forge with.
            false
        };

        match (cross_origin, websocket) {
            (false, _) => OriginVerdict::Allow,
            (true, false) => OriginVerdict::Deny,
            (true, true) => OriginVerdict::UntrustedWebSocket,
        }
    }
}

/// Verifies with no trusted cross-origin peers.
impl Default for OriginPolicy {
    fn default() -> Self {
        Self {
            inner: Inner::Verify {
                trusted_origins: Vec::new(),
            },
        }
    }
}

/// The router's decision about a request's origin.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OriginVerdict {
    /// The request passes.
    Allow,
    /// A state-changing cross-origin browser request; the router rejects it
    /// with `403 Forbidden`.
    Deny,
    /// A cross-origin WebSocket handshake from an untrusted origin. The
    /// router lets it through with [`UntrustedWebSocketOrigin`] recorded on
    /// the request context, and the WebSocket upgrade rejects it unless the
    /// endpoint opted in.
    UntrustedWebSocket,
}

/// Recorded on the request context when a WebSocket handshake arrives from an
/// untrusted cross-origin page. The WebSocket upgrade refuses to open the
/// socket unless the endpoint allows the origin.
#[derive(Debug)]
pub(crate) struct UntrustedWebSocketOrigin;

#[cfg(test)]
mod tests {
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    /// Runs `policy` against a request with the given method and headers.
    fn check(policy: &OriginPolicy, method: Method, headers: &[(&str, &str)]) -> OriginVerdict {
        check_uri(policy, method, "/x", headers)
    }

    fn check_uri(
        policy: &OriginPolicy,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
    ) -> OriginVerdict {
        let mut builder = http::Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        let cx = CxTestBuilder::new().request_context(parts).build();
        policy.check(&cx)
    }

    fn post_verdict(headers: &[(&str, &str)]) -> OriginVerdict {
        check(&OriginPolicy::default(), Method::POST, headers)
    }

    /// The headers of a browser WebSocket handshake from `origin`.
    fn handshake(origin: &'static str) -> Vec<(&'static str, &'static str)> {
        vec![
            ("host", "app.example"),
            ("upgrade", "websocket"),
            ("origin", origin),
        ]
    }

    #[test]
    fn safe_methods_pass_even_cross_site() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                check(
                    &OriginPolicy::default(),
                    method,
                    &[
                        ("host", "app.example"),
                        ("origin", "https://evil.example"),
                        ("sec-fetch-site", "cross-site"),
                    ],
                ),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn same_origin_and_direct_navigation_pass() {
        for site in ["same-origin", "none"] {
            assert_eq!(
                post_verdict(&[("host", "app.example"), ("sec-fetch-site", site)]),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn same_site_and_cross_site_are_rejected() {
        // `same-site` is rejected deliberately: a sibling subdomain must not
        // be able to forge requests.
        assert_eq!(
            post_verdict(&[
                ("host", "app.example"),
                ("origin", "https://evil.app.example"),
                ("sec-fetch-site", "same-site"),
            ]),
            OriginVerdict::Deny
        );
        assert_eq!(
            post_verdict(&[
                ("host", "app.example"),
                ("origin", "https://evil.example"),
                ("sec-fetch-site", "cross-site"),
            ]),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn trusted_origins_pass_even_cross_site() {
        let policy = OriginPolicy::trust_origins(["https://accounts.example"]);
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[
                    ("host", "app.example"),
                    ("origin", "https://ACCOUNTS.example"),
                    ("sec-fetch-site", "cross-site"),
                ],
            ),
            OriginVerdict::Allow
        );
        // Trust is keyed on the `Origin` header; without one the request is
        // still classified by `Sec-Fetch-Site`.
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[("host", "app.example"), ("sec-fetch-site", "cross-site")],
            ),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn origin_fallback_compares_hosts() {
        assert_eq!(
            post_verdict(&[("host", "app.example"), ("origin", "https://app.example")]),
            OriginVerdict::Allow
        );
        // Hosts are case-insensitive, and any explicit port must match.
        assert_eq!(
            post_verdict(&[
                ("host", "app.example:8443"),
                ("origin", "https://App.Example:8443"),
            ]),
            OriginVerdict::Allow
        );
        assert_eq!(
            post_verdict(&[("host", "app.example"), ("origin", "https://evil.example")]),
            OriginVerdict::Deny
        );
        assert_eq!(
            post_verdict(&[
                ("host", "app.example"),
                ("origin", "https://app.example:8443"),
            ]),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn opaque_origin_is_rejected() {
        assert_eq!(
            post_verdict(&[("host", "app.example"), ("origin", "null")]),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn origin_without_a_request_host_is_rejected() {
        assert_eq!(
            post_verdict(&[("origin", "https://app.example")]),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn uri_authority_backs_a_missing_host_header() {
        assert_eq!(
            check_uri(
                &OriginPolicy::default(),
                Method::POST,
                "https://app.example/x",
                &[("origin", "https://app.example")],
            ),
            OriginVerdict::Allow
        );
    }

    #[test]
    fn non_browser_requests_pass() {
        assert_eq!(
            post_verdict(&[("host", "app.example")]),
            OriginVerdict::Allow
        );
        assert_eq!(post_verdict(&[]), OriginVerdict::Allow);
    }

    #[test]
    fn disabled_policy_allows_everything() {
        assert_eq!(
            check(
                &OriginPolicy::dangerous_disable(),
                Method::POST,
                &[
                    ("host", "app.example"),
                    ("origin", "https://evil.example"),
                    ("sec-fetch-site", "cross-site"),
                ],
            ),
            OriginVerdict::Allow
        );
    }

    // -- WebSocket handshakes --

    #[test]
    fn cross_origin_handshakes_are_flagged_not_denied() {
        assert_eq!(
            check(
                &OriginPolicy::default(),
                Method::GET,
                &handshake("https://evil.example"),
            ),
            OriginVerdict::UntrustedWebSocket
        );
    }

    #[test]
    fn same_origin_and_trusted_handshakes_pass() {
        assert_eq!(
            check(
                &OriginPolicy::default(),
                Method::GET,
                &handshake("https://app.example"),
            ),
            OriginVerdict::Allow
        );
        assert_eq!(
            check(
                &OriginPolicy::trust_origins(["https://admin.example"]),
                Method::GET,
                &handshake("https://admin.example"),
            ),
            OriginVerdict::Allow
        );
    }

    #[test]
    fn non_browser_handshakes_pass() {
        assert_eq!(
            check(
                &OriginPolicy::default(),
                Method::GET,
                &[("host", "app.example"), ("upgrade", "websocket")],
            ),
            OriginVerdict::Allow
        );
    }

    #[test]
    fn handshakes_are_detected_by_fetch_metadata_and_upgrade() {
        // `Sec-Fetch-Mode: websocket` alone marks a handshake.
        assert_eq!(
            check(
                &OriginPolicy::default(),
                Method::GET,
                &[
                    ("host", "app.example"),
                    ("origin", "https://evil.example"),
                    ("sec-fetch-mode", "websocket"),
                ],
            ),
            OriginVerdict::UntrustedWebSocket
        );
        // The `Upgrade` header may carry a list of tokens in any casing.
        assert_eq!(
            check(
                &OriginPolicy::default(),
                Method::GET,
                &[
                    ("host", "app.example"),
                    ("origin", "https://evil.example"),
                    ("upgrade", "keep-alive, WebSocket"),
                ],
            ),
            OriginVerdict::UntrustedWebSocket
        );
        // A non-GET request with an `Upgrade` header is not a handshake and
        // falls under the state-changing rules.
        assert_eq!(
            post_verdict(&[
                ("host", "app.example"),
                ("origin", "https://evil.example"),
                ("upgrade", "websocket"),
            ]),
            OriginVerdict::Deny
        );
    }
}
