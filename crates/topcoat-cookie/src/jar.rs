use std::sync::{Mutex, MutexGuard};

use cookie::{Cookie, CookieJar as RawCookieJar};
use http::{HeaderValue, header, request::Parts};
use topcoat_core::runtime::context::{Cx, request_context};

use crate::Cookies;

/// The root cookie jar for a request.
///
/// `CookieJar` wraps the [`cookie`] crate's [`CookieJar`](RawCookieJar) behind a
/// [`Mutex`], giving the whole [`Cookies`] adapter stack interior mutability and
/// thread safety. It is created lazily by [`cookies`](crate::cookies), which
/// parses the incoming `Cookie` header on first access and memoizes the jar for
/// the rest of the request.
///
/// Every adapter ([`SignedJar`](crate::SignedJar), [`PrivateJar`](crate::PrivateJar),
/// [`Prefixed`](crate::Prefixed), [`Map`](crate::Map)) ultimately reads from and
/// writes to this jar, so the pending changes it accumulates are what gets
/// serialized into `Set-Cookie` response headers.
#[derive(Debug)]
pub struct CookieJar {
    jar: Mutex<RawCookieJar>,
}

impl CookieJar {
    /// Builds a jar from the request's `Cookie` header(s), seeding each parsed
    /// cookie as an original (so it does not count towards the response delta).
    ///
    /// Reads the request headers from the [`Parts`] registered in request
    /// context by the router.
    pub(crate) fn from_request(cx: &Cx) -> Self {
        let mut jar = RawCookieJar::new();
        let parts = request_context::<Parts>(cx);
        for value in parts.headers.get_all(header::COOKIE) {
            let Ok(raw) = value.to_str() else { continue };
            for cookie in Cookie::split_parse_encoded(raw.to_owned()).flatten() {
                jar.add_original(cookie);
            }
        }
        Self {
            jar: Mutex::new(jar),
        }
    }

    fn lock(&self) -> MutexGuard<'_, RawCookieJar> {
        self.jar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Renders the jar's pending changes as `Set-Cookie` header values.
    pub(crate) fn delta_headers(&self) -> Vec<HeaderValue> {
        self.lock()
            .delta()
            .filter_map(|cookie| HeaderValue::from_str(&cookie.encoded().to_string()).ok())
            .collect()
    }
}

impl Cookies for &CookieJar {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.lock().get(name).cloned()
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.lock().add(cookie.into());
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.lock().remove(cookie.into());
    }
}
