use topcoat_core::context::Cx;

use crate::{
    Body, IntoPath, Layer, LayerFuture, Method, Next, Path, PathBuf,
    error::forbidden,
    header,
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
/// to trust cross-origin peers, to exempt individual routes, or to opt out of
/// verification.
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

                // Fallback check for older browsers: Compare the "origin" and "host" header
                // manually. If they are the same, this is a safe same-origin
                // request.
                if let Some(origin) = origin {
                    let origin = origin.split_once("://").map(|(_, host)| host);
                    let host = header(header::HOST.as_str())
                        .or_else(|| uri(cx).authority().map(http::uri::Authority::as_str));
                    return if let Some(host) = host
                        && let Some(origin) = origin
                        && origin.eq_ignore_ascii_case(host)
                    {
                        OriginVerdict::Allow
                    } else {
                        OriginVerdict::Deny
                    };
                }

                // Ther request had neither an "origin" nor a "sec-fetch-site" header.
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
    fn path(&self) -> &Path {
        Path::ROOT
    }

    fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        match self.policy.check(cx) {
            OriginVerdict::Allow => next.run(cx, body),
            OriginVerdict::Deny => Box::pin(async { Err(forbidden().into()) }),
        }
    }
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

    // -- the origin/host fallback for older browsers --

    #[test]
    fn matching_origin_and_host_pass() {
        let policy = OriginPolicy::new();
        assert_eq!(
            check(
                &policy,
                Method::POST,
                &[("origin", "https://example.com"), ("host", "EXAMPLE.com")]
            ),
            OriginVerdict::Allow
        );
    }

    #[test]
    fn mismatched_origin_and_host_are_denied_for_unsafe_methods() {
        let policy = OriginPolicy::new();
        let headers = [("origin", "https://evil.example"), ("host", "example.com")];
        assert_eq!(check(&policy, Method::POST, &headers), OriginVerdict::Deny);
        assert_eq!(check(&policy, Method::GET, &headers), OriginVerdict::Allow);
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
