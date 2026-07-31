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
    #[must_use]
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
        match &self.inner {
            Inner::Disabled => OriginVerdict::Allow,
            Inner::Verify { trusted_origins } => {
                let method = method(cx);
                let headers = headers(cx);
                let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
                let origin = header(header::ORIGIN.as_str());
                let host = header(header::HOST.as_str())
                    .or_else(|| uri(cx).authority().map(http::uri::Authority::as_str));

                // An explicitly trusted origin passes regardless of how the browser
                // classified the request.
                if origin.is_some_and(|origin| {
                    trusted_origins
                        .iter()
                        .any(|trusted| trusted.eq_ignore_ascii_case(origin))
                }) {
                    return OriginVerdict::Allow;
                }

                // The "sec-fetch-site" header is the browser's way to declare that the request comes from
                // the same origin. If so, it is safe.
                if let Some(site) = header("sec-fetch-site") {
                    return if matches!(site, "same-origin" | "none") {
                        OriginVerdict::Allow
                    } else {
                        OriginVerdict::from_method(method)
                    };
                }

                // Fallback check for older browsers: Compare the "origin" and "host" header manually.
                // If they are the same, this is a safe same-origin request.
                if let Some(origin) = origin {
                    let origin = origin.split_once("://").map(|(_, host)| host);
                    return if let Some(host) = host
                        && let Some(origin) = origin
                        && origin.eq_ignore_ascii_case(host)
                    {
                        OriginVerdict::Allow
                    } else {
                        OriginVerdict::from_method(method)
                    };
                }

                // Ther request had neither an "origin" nor a "sec-fetch-site" header.
                // We assume it comes from a non-browser client and allow the request.
                OriginVerdict::Allow
            }
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
    /// A state-changing request from an untrusted origin.
    Deny,
    /// A non-state-changing request from an untrusted origin.
    /// This can be used, for example, to reject WebSocket upgrades.
    Untrusted,
}

impl OriginVerdict {
    fn from_method(method: &Method) -> Self {
        match method {
            &Method::GET | &Method::HEAD | &Method::OPTIONS => OriginVerdict::Untrusted,
            _ => OriginVerdict::Deny,
        }
    }
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    /// The request URI used by every test that does not exercise the fallback
    /// to the URI's authority.
    const URI: &str = "/checkout";

    /// The `Host` header of the application being protected.
    const HOST: (&str, &str) = ("host", "app.example");

    /// Builds a `Cx` for a request with the given method, URI, and headers.
    fn cx_with(method: Method, uri: &str, headers: &[(&str, &str)]) -> Cx {
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    /// The verdict `policy` reaches for a request with the given method and
    /// headers.
    fn verdict(policy: &OriginPolicy, method: Method, headers: &[(&str, &str)]) -> OriginVerdict {
        policy.check(&cx_with(method, URI, headers))
    }

    /// The verdict the default policy reaches for the given method and headers.
    fn default_verdict(method: Method, headers: &[(&str, &str)]) -> OriginVerdict {
        verdict(&OriginPolicy::default(), method, headers)
    }

    // -- sec-fetch-site --

    #[test]
    fn same_origin_and_direct_navigation_pass() {
        for site in ["same-origin", "none"] {
            for method in [Method::GET, Method::POST] {
                assert_eq!(
                    default_verdict(method, &[HOST, ("sec-fetch-site", site)]),
                    OriginVerdict::Allow,
                );
            }
        }
    }

    #[test]
    fn cross_site_state_changing_requests_are_denied() {
        for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
            assert_eq!(
                default_verdict(
                    method,
                    &[
                        HOST,
                        ("sec-fetch-site", "cross-site"),
                        ("origin", "https://evil.example"),
                    ],
                ),
                OriginVerdict::Deny,
            );
        }
    }

    #[test]
    fn cross_site_safe_requests_are_untrusted() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                default_verdict(
                    method,
                    &[
                        HOST,
                        ("sec-fetch-site", "cross-site"),
                        ("origin", "https://evil.example"),
                    ],
                ),
                OriginVerdict::Untrusted,
            );
        }
    }

    #[test]
    fn same_site_is_rejected() {
        // `same-site` is rejected deliberately: a sibling subdomain must not be
        // able to forge requests.
        let headers = [
            HOST,
            ("sec-fetch-site", "same-site"),
            ("origin", "https://evil.app.example"),
        ];
        assert_eq!(default_verdict(Method::POST, &headers), OriginVerdict::Deny);
        assert_eq!(
            default_verdict(Method::GET, &headers),
            OriginVerdict::Untrusted,
        );
    }

    // -- trusted origins --

    #[test]
    fn trusted_origins_pass_even_cross_site() {
        let policy = OriginPolicy::trust_origins(["https://accounts.example"]);
        for method in [Method::GET, Method::POST] {
            assert_eq!(
                verdict(
                    &policy,
                    method,
                    &[
                        HOST,
                        ("sec-fetch-site", "cross-site"),
                        ("origin", "https://accounts.example"),
                    ],
                ),
                OriginVerdict::Allow,
            );
        }
    }

    #[test]
    fn trusted_origins_are_compared_case_insensitively() {
        let policy = OriginPolicy::trust_origins(["https://Accounts.Example"]);
        assert_eq!(
            verdict(
                &policy,
                Method::POST,
                &[
                    HOST,
                    ("sec-fetch-site", "cross-site"),
                    ("origin", "https://accounts.example"),
                ],
            ),
            OriginVerdict::Allow,
        );
    }

    #[test]
    fn trust_is_keyed_on_the_origin_header() {
        let policy = OriginPolicy::trust_origins(["https://accounts.example"]);
        // Another origin is still classified by `Sec-Fetch-Site`.
        assert_eq!(
            verdict(
                &policy,
                Method::POST,
                &[
                    HOST,
                    ("sec-fetch-site", "cross-site"),
                    ("origin", "https://evil.example"),
                ],
            ),
            OriginVerdict::Deny,
        );
        // Without an `Origin` header there is nothing to match against.
        assert_eq!(
            verdict(
                &policy,
                Method::POST,
                &[HOST, ("sec-fetch-site", "cross-site")],
            ),
            OriginVerdict::Deny,
        );
    }

    // -- origin fallback for older browsers --

    #[test]
    fn origin_fallback_compares_hosts() {
        assert_eq!(
            default_verdict(Method::POST, &[HOST, ("origin", "https://app.example")]),
            OriginVerdict::Allow,
        );
        // Hosts are compared case-insensitively, and any explicit port is part
        // of the comparison.
        assert_eq!(
            default_verdict(
                Method::POST,
                &[
                    ("host", "app.example:8443"),
                    ("origin", "https://App.Example:8443"),
                ],
            ),
            OriginVerdict::Allow,
        );
        assert_eq!(
            default_verdict(
                Method::POST,
                &[HOST, ("origin", "https://app.example:8443")],
            ),
            OriginVerdict::Deny,
        );
        assert_eq!(
            default_verdict(Method::POST, &[HOST, ("origin", "https://evil.example")]),
            OriginVerdict::Deny,
        );
        assert_eq!(
            default_verdict(Method::GET, &[HOST, ("origin", "https://evil.example")]),
            OriginVerdict::Untrusted,
        );
    }

    #[test]
    fn opaque_origins_are_rejected() {
        // `Origin: null` carries no host, so it can never match the request's
        // own and must not be mistaken for a non-browser client.
        assert_eq!(
            default_verdict(Method::POST, &[HOST, ("origin", "null")]),
            OriginVerdict::Deny,
        );
        assert_eq!(
            default_verdict(Method::GET, &[HOST, ("origin", "null")]),
            OriginVerdict::Untrusted,
        );
    }

    #[test]
    fn an_origin_without_a_request_host_is_rejected() {
        assert_eq!(
            default_verdict(Method::POST, &[("origin", "https://app.example")]),
            OriginVerdict::Deny,
        );
    }

    #[test]
    fn the_host_falls_back_to_the_uri_authority() {
        let policy = OriginPolicy::default();
        let cx = cx_with(
            Method::POST,
            "https://app.example/checkout",
            &[("origin", "https://app.example")],
        );
        assert_eq!(policy.check(&cx), OriginVerdict::Allow);

        // The `Host` header wins over the URI's authority.
        let cx = cx_with(
            Method::POST,
            "https://other.example/checkout",
            &[HOST, ("origin", "https://app.example")],
        );
        assert_eq!(policy.check(&cx), OriginVerdict::Allow);
    }

    // -- non-browser clients --

    #[test]
    fn requests_without_either_header_pass() {
        assert_eq!(default_verdict(Method::POST, &[HOST]), OriginVerdict::Allow);
        assert_eq!(default_verdict(Method::POST, &[]), OriginVerdict::Allow);
    }

    // -- disabled verification --

    #[test]
    fn disabling_verification_allows_everything() {
        let policy = OriginPolicy::dangerous_disable();
        for method in [Method::GET, Method::POST] {
            assert_eq!(
                verdict(
                    &policy,
                    method,
                    &[
                        HOST,
                        ("sec-fetch-site", "cross-site"),
                        ("origin", "https://evil.example"),
                    ],
                ),
                OriginVerdict::Allow,
            );
        }
    }
}
