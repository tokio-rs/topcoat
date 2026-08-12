use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
};

use cookie::{Cookie, CookieJar as RawCookieJar};
use http::{HeaderValue, header, request::Parts};
use topcoat_core::context::{Cx, request_context};

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
///
/// Once those headers are written the jar is sealed: reads keep working, but
/// adding or removing a cookie panics instead of being silently dropped.
#[derive(Debug)]
pub struct CookieJar {
    jar: Mutex<RawCookieJar>,
    sealed: AtomicBool,
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
            sealed: AtomicBool::new(false),
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

    /// Closes the jar for writing, after its changes have been written onto the
    /// response.
    pub(crate) fn seal(&self) {
        self.sealed.store(true, Ordering::Release);
    }

    /// Panics if the response has already taken this jar's changes, because a
    /// write made now would never reach the client.
    fn assert_open(&self, action: &str) {
        assert!(
            !self.sealed.load(Ordering::Acquire),
            "cannot {action} a cookie after the response headers have been sent. \
             Pending cookies are written when the handler returns, so work that \
             outlives it, such as a streaming body or a WebSocket task, can only \
             read cookies"
        );
    }
}

impl Cookies for &CookieJar {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.lock().get(name).cloned()
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.assert_open("add");
        self.lock().add(cookie.into());
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.assert_open("remove");
        self.lock().remove(cookie.into());
    }
}
