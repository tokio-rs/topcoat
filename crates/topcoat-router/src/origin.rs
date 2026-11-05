use topcoat_core::context::{Cx, try_context};

use crate::{
    Method,
    error::{ForbiddenError, forbidden},
    header,
    request::{headers, method, uri},
};

/// The cross-origin request policy the router applies to a request.
///
/// By default, rejects state-changing browser requests and WebSocket
/// handshakes from untrusted origins. This helps protect against cross-site
/// request forgery and cross-site WebSocket hijacking.
///
/// Requests identified as same-origin or direct navigations pass. Requests
/// without origin or fetch metadata headers also pass. A rejected request
/// fails with `403 Forbidden` before any layer or route runs.
///
/// The router reads the policy from the request context. Pass a policy to
/// [`RouterBuilder::with`](crate::RouterBuilder::with) to replace the default
/// for the whole application, or to `with(...)` on a route, page, layout, or
/// layer to replace it for that part of the application.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{OriginPolicy, Router, route},
/// };
///
/// // Accepts requests from any site and verifies their signature itself.
/// #[route(POST "/webhook", with(OriginPolicy::dangerous_disable()))]
/// async fn webhook(cx: &Cx) -> Result<()> {
///     Ok(())
/// }
///
/// let router = Router::builder()
///     .with(OriginPolicy::trust_origins([
///         "https://accounts.example.com",
///     ]))
///     .route(webhook)
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
    /// Creates a policy that verifies requests like the default policy, but
    /// also trusts `origins` to send state-changing cross-origin requests and
    /// to open cross-origin WebSockets.
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

    /// Creates a policy that turns origin verification off.
    ///
    /// Without verification, nothing rejects state-changing cross-origin
    /// requests or cross-origin WebSocket handshakes. Only use this where the
    /// application enforces its own defense against cross-site request
    /// forgery, like a webhook that verifies a signature.
    #[must_use]
    pub fn dangerous_disable() -> Self {
        Self {
            inner: Inner::Disabled,
        }
    }

    /// Checks the current request against this policy and returns a
    /// forbidden error if the policy rejects it.
    ///
    /// The router already checks every request against the policy from
    /// [`origin_policy`] before any layer or route runs. Call this to apply a
    /// policy on your own terms, like in a route that disables the check for
    /// signed requests only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn has_valid_signature(_cx: &Cx) -> bool { true }
    /// use topcoat::{
    ///     Result,
    ///     context::Cx,
    ///     router::{OriginPolicy, route},
    /// };
    ///
    /// #[route(POST "/webhook", with(OriginPolicy::dangerous_disable()))]
    /// async fn webhook(cx: &Cx) -> Result<()> {
    ///     if !has_valid_signature(cx) {
    ///         OriginPolicy::default().verify(cx)?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ForbiddenError`] if the policy rejects the request.
    pub fn verify(&self, cx: &Cx) -> Result<(), ForbiddenError> {
        match self.check(cx) {
            OriginVerdict::Allow => Ok(()),
            OriginVerdict::Deny => Err(forbidden()),
        }
    }

    /// Classifies the current request under this policy.
    fn check(&self, cx: &Cx) -> OriginVerdict {
        match &self.inner {
            Inner::Disabled => OriginVerdict::Allow,
            Inner::Verify { trusted_origins } => {
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

                // The request had neither an "origin" nor a "sec-fetch-site" header.
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

/// The policy that applies when the context holds none.
static DEFAULT_POLICY: OriginPolicy = OriginPolicy {
    inner: Inner::Verify {
        trusted_origins: Vec::new(),
    },
};

/// Returns the [`OriginPolicy`] that applies to the request.
///
/// This is the policy from the most specific `with(...)` that covers the
/// matched route, the policy passed to
/// [`RouterBuilder::with`](crate::RouterBuilder::with), or the default policy
/// if there is neither.
#[must_use]
pub fn origin_policy(cx: &Cx) -> &OriginPolicy {
    try_context(cx).unwrap_or(&DEFAULT_POLICY)
}

/// The policy's decision about a request's origin.
#[derive(Debug, PartialEq, Eq)]
enum OriginVerdict {
    /// The request passes.
    Allow,
    /// A dangerous request from an untrusted origin.
    Deny,
}

#[cfg(test)]
mod tests {
    use http::{Request, StatusCode};
    use topcoat::view::view;
    use topcoat_core::context::Cx;
    use topcoat_view::{BoxView, ViewExt};

    use super::*;
    use crate::{
        Body, LayoutFn, PageFn, RouteFn, RouteFuture, Router, Slot, response::IntoResponse,
    };

    /// Classifies a request with the given method and headers under `policy`.
    fn check(policy: &OriginPolicy, method: Method, headers: &[(&str, &str)]) -> OriginVerdict {
        let mut builder = Request::builder().method(method).uri("/x");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        policy.check(&Cx::new().with(parts))
    }

    // -- default policy --

    #[test]
    fn non_browser_requests_pass() {
        // Neither an "origin" nor a "sec-fetch-site" header.
        let policy = OriginPolicy::default();
        assert_eq!(check(&policy, Method::POST, &[]), OriginVerdict::Allow);
    }

    #[test]
    fn same_origin_requests_pass() {
        let policy = OriginPolicy::default();
        for site in ["same-origin", "none"] {
            assert_eq!(
                check(&policy, Method::POST, &[("sec-fetch-site", site)]),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn cross_origin_safe_methods_pass() {
        let policy = OriginPolicy::default();
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                check(&policy, method, &[("sec-fetch-site", "cross-site")]),
                OriginVerdict::Allow
            );
        }
    }

    #[test]
    fn cross_origin_unsafe_methods_are_denied() {
        let policy = OriginPolicy::default();
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
        let policy = OriginPolicy::default();
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
        let policy = OriginPolicy::default();
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
        let policy = OriginPolicy::default();
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
        let policy = OriginPolicy::default();
        let headers = [("origin", "https://evil.example"), ("host", "example.com")];
        assert_eq!(check(&policy, Method::POST, &headers), OriginVerdict::Deny);
        assert_eq!(check(&policy, Method::GET, &headers), OriginVerdict::Allow);
    }

    // -- trusted origins --

    #[test]
    fn trusted_origins_pass() {
        let policy = OriginPolicy::trust_origins(["https://accounts.example.com"]);
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

    // -- the policy through the router --

    /// Answers every request with `200 OK`.
    fn ok(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "ok".into_response(cx) })
    }

    /// Renders an empty page.
    fn empty_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {}.boxed()
    }

    /// Renders the page inside the layout.
    fn passthrough_layout<'a>(cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a> {
        view! { (slot) }.boxed()
    }

    /// Sends a cross-site POST from `https://peer.example` to `path`.
    async fn send_cross_site(router: &Router, path: &str) -> StatusCode {
        let request = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header("origin", "https://peer.example")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .expect("request should build");
        router.handle(request).await.status()
    }

    #[tokio::test]
    async fn the_default_policy_applies_without_registration() {
        let router = Router::builder()
            .route(RouteFn::new(Method::POST, "/x", ok))
            .build();
        assert_eq!(send_cross_site(&router, "/x").await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn unmatched_requests_are_verified() {
        // The check runs before the 404 resolves, so an untrusted cross-origin
        // request learns nothing about which paths exist.
        let router = Router::builder()
            .route(RouteFn::new(Method::POST, "/x", ok))
            .build();
        assert_eq!(
            send_cross_site(&router, "/missing").await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_router_policy_applies_to_every_route() {
        let router = Router::builder()
            .with(OriginPolicy::trust_origins(["https://peer.example"]))
            .route(RouteFn::new(Method::POST, "/x", ok))
            .build();
        assert_eq!(send_cross_site(&router, "/x").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_route_policy_replaces_the_router_policy() {
        let router = Router::builder()
            .with(OriginPolicy::trust_origins(["https://peer.example"]))
            .route(RouteFn::new(Method::POST, "/strict", ok).with(OriginPolicy::default()))
            .route(RouteFn::new(Method::POST, "/x", ok))
            .build();
        assert_eq!(
            send_cross_site(&router, "/strict").await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(send_cross_site(&router, "/x").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_route_policy_does_not_apply_to_unmatched_requests() {
        let router = Router::builder()
            .route(RouteFn::new(Method::POST, "/x", ok).with(OriginPolicy::dangerous_disable()))
            .build();
        assert_eq!(send_cross_site(&router, "/x").await, StatusCode::OK);
        assert_eq!(
            send_cross_site(&router, "/missing").await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_layout_policy_applies_to_the_pages_under_it() {
        let router = Router::builder()
            .layout(
                LayoutFn::new("/feed", passthrough_layout).with(OriginPolicy::dangerous_disable()),
            )
            .page(PageFn::new(Method::POST, "/feed/items", empty_page))
            .page(PageFn::new(Method::POST, "/other", empty_page))
            .build();
        assert_eq!(
            send_cross_site(&router, "/feed/items").await,
            StatusCode::OK
        );
        assert_eq!(
            send_cross_site(&router, "/other").await,
            StatusCode::FORBIDDEN
        );
    }
}
