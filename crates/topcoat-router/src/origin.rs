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
        match &self.inner {
            Inner::Disabled => OriginVerdict::Allow,
            Inner::Verify { trusted_origins } => {
                let headers = headers(cx);
                let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
                let origin = header(header::ORIGIN.as_str());

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
                if let Some(site) = header("sec-fetch-site")
                    && matches!(site, "same-origin" | "none")
                {
                    return OriginVerdict::Allow;
                }

                // Fallback check for older browsers: Compare the "origin" and "host" header manually.
                // If they are the same, this is a safe same-origin request.
                {
                    let origin =
                        origin.and_then(|origin| origin.split_once("://").map(|(_, host)| host));
                    let host = header(header::HOST.as_str())
                        .or_else(|| uri(cx).authority().map(http::uri::Authority::as_str));
                    if let Some(origin) = origin
                        && let Some(host) = host
                        && origin.eq_ignore_ascii_case(host)
                    {
                        return OriginVerdict::Allow;
                    }
                }

                // We now know the request is from an untrusted source. We still permit non-state-changing
                // requests but with `Untrusted` verdict. The `Untrusted` verdict can be used by
                // WebSockets to reject the request during an upgrade.
                match *method(cx) {
                    Method::GET | Method::HEAD | Method::OPTIONS => OriginVerdict::Untrusted,
                    _ => OriginVerdict::Deny,
                }
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
    Untrusted,
}
