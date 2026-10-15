use http::Uri;
use topcoat_core::{base_url::try_base_url, context::Cx};

use crate::{
    Body, IntoPath, Layer, LayerFuture, Method, Next, Path, PathBuf,
    error::forbidden,
    header,
    request::{headers, method, uri},
};

/// The cross-origin request policy the router applies to every request.
///
/// By default, rejects state-changing browser requests and WebSocket
/// handshakes from untrusted origins. This helps protect against cross-site
/// request forgery and cross-site WebSocket hijacking.
///
/// Requests identified as same-origin or direct navigations pass. Requests
/// without origin or fetch metadata headers also pass. Register a policy with
/// [`RouterBuilder::origin_policy`](crate::RouterBuilder::origin_policy) to
/// trust origins, exempt paths, or disable verification.
///
/// Without fetch metadata, an `Origin` header must match the scheme, host,
/// and port of the configured [`base_url`](crate::RouterBuilder::base_url),
/// or the absolute request URI when no base URL is configured. Without a
/// base URL, a path-only request URI cannot establish the scheme, so these
/// requests are rejected unless the origin is explicitly trusted. Configure
/// the public base URL behind a reverse proxy; forwarding headers are not
/// used for this check.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{OriginPolicy, Router};
///
/// let router = Router::builder()
///     .origin_policy(OriginPolicy::new().trust_origins(["https://accounts.example.com"]))
///     .build();
/// ```
#[derive(Debug)]
pub struct OriginPolicy {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    /// Verifies every request, trusting the listed cross-origin peers and
    /// skipping the exempted paths.
    Verify {
        trusted_origins: Vec<String>,
        exempt_paths: Vec<PathBuf>,
    },
    /// No verification at all.
    Disabled,
}

impl OriginPolicy {
    /// Creates the default policy: every request is verified, with no trusted
    /// cross-origin peers and no exempt paths.
    ///
    /// Loosen it with [`trust_origins`](Self::trust_origins) and
    /// [`exempt_paths`](Self::exempt_paths).
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Inner::Verify {
                trusted_origins: Vec::new(),
                exempt_paths: Vec::new(),
            },
        }
    }

    /// Trusts `origins` to send state-changing cross-origin requests and to
    /// open cross-origin WebSockets.
    ///
    /// Each value is compared against the request's `Origin` header, so pass
    /// the full serialized origin: scheme, host, and any non-default port
    /// (`"https://accounts.example.com"`), with no trailing slash. The
    /// comparison is ASCII case-insensitive.
    #[must_use]
    pub fn trust_origins<I>(mut self, origins: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        if let Inner::Verify {
            trusted_origins, ..
        } = &mut self.inner
        {
            trusted_origins.extend(origins.into_iter().map(Into::into));
        }
        self
    }

    /// Exempts the routes at `paths` from origin verification.
    ///
    /// A request whose URL matches one of the paths passes unchecked, no
    /// matter where it comes from. Use this for a route that must accept
    /// cross-origin requests from anywhere and handles its own protection,
    /// like a WebSocket endpoint open to other sites. Paths use the route
    /// path syntax, so `{param}` and `{*rest}` segments match like a route.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::OriginPolicy;
    ///
    /// let policy = OriginPolicy::new().exempt_paths(["/feed/{*rest}"]);
    /// ```
    #[must_use]
    pub fn exempt_paths<I>(mut self, paths: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoPath,
    {
        if let Inner::Verify { exempt_paths, .. } = &mut self.inner {
            exempt_paths.extend(paths.into_iter().map(|path| path.into_path().into_owned()));
        }
        self
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
    fn check(&self, cx: &Cx) -> OriginVerdict {
        match &self.inner {
            Inner::Disabled => OriginVerdict::Allow,
            Inner::Verify {
                trusted_origins,
                exempt_paths,
            } => {
                let headers = headers(cx);
                let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());

                // Only a request that can do damage needs verification: any
                // method beyond the safe ones, and a WebSocket handshake, which
                // arrives as a GET but opens a credentialed two-way connection.
                let upgrades_to_websocket = header(header::UPGRADE.as_str())
                    .is_some_and(|value| value.trim().eq_ignore_ascii_case("websocket"));
                if !upgrades_to_websocket
                    && matches!(method(cx), &Method::GET | &Method::HEAD | &Method::OPTIONS)
                {
                    return OriginVerdict::Allow;
                }

                // An exempted route handles cross-origin requests itself.
                if exempt_paths.iter().any(|path| path.matches(uri(cx).path())) {
                    return OriginVerdict::Allow;
                }

                // An explicitly trusted origin passes regardless of how the browser
                // classified the request.
                let origin = header(header::ORIGIN.as_str());
                if origin.is_some_and(|origin| {
                    trusted_origins
                        .iter()
                        .any(|trusted| trusted.eq_ignore_ascii_case(origin))
                }) {
                    return OriginVerdict::Allow;
                }

                // The "sec-fetch-site" header is the browser's way to declare that the request
                // comes from the same origin. If so, it is safe.
                if let Some(site) = header("sec-fetch-site") {
                    return if matches!(site, "same-origin" | "none") {
                        OriginVerdict::Allow
                    } else {
                        OriginVerdict::Deny
                    };
                }

                // Without fetch metadata, compare the complete public origin.
                // A Host header alone cannot establish the request's scheme.
                if let Some(origin) = origin {
                    let target = match try_base_url(cx) {
                        Some(base) => base.as_str().parse().expect("base URL is a valid URI"),
                        None => uri(cx).clone(),
                    };
                    return if same_origin(origin, &target) {
                        OriginVerdict::Allow
                    } else {
                        OriginVerdict::Deny
                    };
                }

                // The request had neither an "origin" nor a "sec-fetch-site" header.
                // We assume it comes from a non-browser client and allow the request.
                OriginVerdict::Allow
            }
        }
    }
}

/// Verifies with no trusted cross-origin peers and no exempt paths.
impl Default for OriginPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// The policy's decision about a request's origin.
#[derive(Debug, PartialEq, Eq)]
enum OriginVerdict {
    /// The request passes.
    Allow,
    /// A dangerous request from an untrusted origin.
    Deny,
}

/// A [`Layer`] enforcing an [`OriginPolicy`].
///
/// The router wraps one around every request as its outermost step, built
/// from the policy registered with
/// [`RouterBuilder::origin_policy`](crate::RouterBuilder::origin_policy). A
/// request the policy denies is rejected with `403 Forbidden` before any
/// inner layer or route runs.
#[derive(Debug)]
pub struct OriginLayer {
    policy: OriginPolicy,
}

impl OriginLayer {
    /// Creates a layer enforcing `policy`.
    #[must_use]
    pub fn new(policy: OriginPolicy) -> Self {
        Self { policy }
    }
}

impl Layer for OriginLayer {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        match self.policy.check(cx) {
            OriginVerdict::Allow => next.run(cx, body),
            OriginVerdict::Deny => Box::pin(async { Err(forbidden().into()) }),
        }
    }
}

/// Compares a serialized Origin header with a request or public base URL.
fn same_origin(origin: &str, target: &Uri) -> bool {
    let Some((_, authority)) = origin.split_once("://") else {
        return false;
    };
    // An Origin header contains no user information, path, query, or fragment.
    if authority.contains(['@', '/', '?', '#']) {
        return false;
    }
    let Ok(origin) = origin.parse::<Uri>() else {
        return false;
    };
    let (Some((scheme, host, port)), Some((target_scheme, target_host, target_port))) =
        (origin_parts(&origin), origin_parts(target))
    else {
        return false;
    };
    scheme.eq_ignore_ascii_case(target_scheme)
        && host.eq_ignore_ascii_case(target_host)
        && port == target_port
}

/// Returns an HTTP origin's scheme, host, and effective port.
fn origin_parts(uri: &Uri) -> Option<(&str, &str, u16)> {
    let scheme = uri.scheme_str()?;
    let default_port = if scheme.eq_ignore_ascii_case("http") {
        80
    } else if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        return None;
    };
    let authority = uri.authority()?;
    let host = authority.host();
    // Reject invalid explicit ports rather than treating them as absent.
    let port = match authority.as_str().strip_prefix(host)? {
        "" => default_port,
        suffix => {
            let port = suffix.strip_prefix(':')?;
            if !port.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            port.parse().ok()?
        }
    };
    Some((scheme, host, port))
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    /// Builds a `Cx` for a request with the given method, URL, and headers.
    fn cx_with(method: Method, uri: &str, headers: &[(&str, &str)]) -> Cx {
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    fn check(policy: &OriginPolicy, method: Method, headers: &[(&str, &str)]) -> OriginVerdict {
        policy.check(&cx_with(method, "/x", headers))
    }

    // -- default policy --

    #[test]
    fn non_browser_requests_pass() {
        // Neither an "origin" nor a "sec-fetch-site" header.
        let policy = OriginPolicy::new();
        assert_eq!(check(&policy, Method::POST, &[]), OriginVerdict::Allow);
    }

    #[test]
    fn same_origin_requests_pass() {
        let policy = OriginPolicy::new();
        for site in ["same-origin", "none"] {
            assert_eq!(
                check(&policy, Method::POST, &[("sec-fetch-site", site)]),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn cross_origin_safe_methods_pass() {
        let policy = OriginPolicy::new();
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                check(&policy, method, &[("sec-fetch-site", "cross-site")]),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn cross_origin_unsafe_methods_are_denied() {
        let policy = OriginPolicy::new();
        for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
            assert_eq!(
                check(&policy, method, &[("sec-fetch-site", "cross-site")]),
                OriginVerdict::Deny
            );
        }
    }

    #[test]
    fn cross_origin_websocket_upgrades_are_denied() {
        // The handshake is a GET, but upgrading makes it dangerous.
        let policy = OriginPolicy::new();
        assert_eq!(
            check(
                &policy,
                Method::GET,
                &[("sec-fetch-site", "cross-site"), ("upgrade", "WebSocket")]
            ),
            OriginVerdict::Deny
        );
    }

    #[test]
    fn same_origin_websocket_upgrades_pass() {
        let policy = OriginPolicy::new();
        assert_eq!(
            check(
                &policy,
                Method::GET,
                &[("sec-fetch-site", "same-origin"), ("upgrade", "websocket")]
            ),
            OriginVerdict::Allow
        );
    }

    // -- the Origin fallback for older browsers --

    #[test]
    fn cross_scheme_origins_are_denied_without_fetch_metadata() {
        let policy = OriginPolicy::new();
        for (target, origin) in [
            ("https://example.com/x", "http://example.com"),
            ("http://example.com/x", "https://example.com"),
            ("https://example.com:8443/x", "http://example.com:8443"),
            ("http://example.com:8443/x", "https://example.com:8443"),
        ] {
            for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
                let cx = cx_with(
                    method,
                    target,
                    &[("origin", origin), ("host", "example.com")],
                );
                assert_eq!(policy.check(&cx), OriginVerdict::Deny);
            }
            let cx = cx_with(
                Method::GET,
                target,
                &[
                    ("origin", origin),
                    ("host", "example.com"),
                    ("upgrade", "websocket"),
                ],
            );
            assert_eq!(policy.check(&cx), OriginVerdict::Deny);
        }
    }

    #[test]
    fn matching_origins_pass() {
        let policy = OriginPolicy::new();
        for (target, origin) in [
            ("https://example.com/x", "https://EXAMPLE.com"),
            ("http://example.com/x", "http://example.com"),
            ("https://example.com:443/x", "https://example.com"),
            ("http://example.com/x", "http://example.com:80"),
            ("https://example.com:8443/x", "https://example.com:8443"),
            ("https://[::1]:443/x", "https://[::1]"),
        ] {
            let cx = cx_with(Method::POST, target, &[("origin", origin)]);
            assert_eq!(policy.check(&cx), OriginVerdict::Allow, "{origin}");
        }
    }

    #[test]
    fn mismatched_hosts_and_ports_are_denied_for_unsafe_methods() {
        let policy = OriginPolicy::new();
        for origin in ["https://evil.example", "https://example.com:8443"] {
            let headers = [("origin", origin), ("host", "example.com")];
            let cx = cx_with(Method::POST, "https://example.com/x", &headers);
            assert_eq!(policy.check(&cx), OriginVerdict::Deny);
            let cx = cx_with(Method::GET, "https://example.com/x", &headers);
            assert_eq!(policy.check(&cx), OriginVerdict::Allow);
        }
    }

    #[test]
    fn absolute_request_schemes_are_case_insensitive() {
        // Parsing a complete URI normalizes HTTP schemes, but the builder
        // preserves their case.
        for (scheme, origin) in [
            ("HTTP", "http://example.com"),
            ("HTTPS", "https://example.com"),
        ] {
            let target = Uri::builder()
                .scheme(scheme)
                .authority("example.com")
                .path_and_query("/x")
                .build()
                .unwrap();
            assert!(same_origin(origin, &target));
        }
    }

    #[test]
    fn a_host_or_forwarded_header_cannot_establish_the_scheme() {
        let policy = OriginPolicy::new();
        for origin in ["http://example.com", "https://example.com"] {
            let headers = [("origin", origin), ("host", "example.com")];
            assert_eq!(check(&policy, Method::POST, &headers), OriginVerdict::Deny);
            let headers = [
                ("origin", origin),
                ("host", "example.com"),
                ("x-forwarded-proto", "https"),
                ("forwarded", "proto=https;host=example.com"),
            ];
            assert_eq!(check(&policy, Method::POST, &headers), OriginVerdict::Deny);
        }
    }

    #[test]
    fn malformed_and_non_http_origins_are_denied() {
        let policy = OriginPolicy::new();
        for origin in [
            "null",
            "example.com",
            "ftp://example.com",
            "https://example.com/",
            "https://example.com?query",
            "https://example.com#fragment",
            "https://user@example.com",
            "https://example.com:invalid",
            "https://example.com:+443",
            "https://example.com:65536",
            "https://example.com:",
        ] {
            let cx = cx_with(Method::POST, "https://example.com/x", &[("origin", origin)]);
            assert_eq!(policy.check(&cx), OriginVerdict::Deny, "{origin}");
        }
    }

    // -- trusted origins --

    #[test]
    fn trusted_origins_pass() {
        let policy = OriginPolicy::new().trust_origins(["https://accounts.example.com"]);
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[
                    ("origin", "https://ACCOUNTS.example.com"),
                    ("sec-fetch-site", "cross-site"),
                ]
            ),
            OriginVerdict::Allow
        );
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[
                    ("origin", "https://other.example.com"),
                    ("sec-fetch-site", "cross-site"),
                ]
            ),
            OriginVerdict::Deny
        );
    }

    // -- exempt paths --

    #[test]
    fn exempt_paths_pass_unchecked() {
        let policy = OriginPolicy::new().exempt_paths(["/ws/{id}"]);
        let headers = [("sec-fetch-site", "cross-site"), ("upgrade", "websocket")];
        assert_eq!(
            policy.check(&cx_with(Method::GET, "/ws/42", &headers)),
            OriginVerdict::Allow
        );
        // Other paths are still verified.
        assert_eq!(
            policy.check(&cx_with(Method::GET, "/other", &headers)),
            OriginVerdict::Deny
        );
    }

    // -- disabled policy --

    #[test]
    fn disabled_policy_passes_everything() {
        let policy = OriginPolicy::dangerous_disable();
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[("sec-fetch-site", "cross-site"), ("upgrade", "websocket")]
            ),
            OriginVerdict::Allow
        );
    }
}
