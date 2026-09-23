use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
};

use cookie::{Cookie, CookieJar as RawCookieJar};
use http::{HeaderValue, header, request::Parts};
use topcoat_core::context::{Cx, request_context};

use crate::Cookies;

/// The root cookie jar of a request.
///
/// Get it with [`cookies`](crate::cookies) and use it through the [`Cookies`]
/// trait, which `&CookieJar` implements. It holds the cookies the request
/// carried and the changes made during the request. The changes are sent as
/// `Set-Cookie` response headers when the handler returns. All adapters built
/// on top of it read from and write to this jar.
///
/// After the response headers are written, reads keep working, but adding or
/// removing a cookie panics, because the change could never reach the client.
#[derive(Debug)]
pub struct CookieJar {
    jar: Mutex<RawCookieJar>,
    sealed: AtomicBool,
}

impl CookieJar {
    /// Builds a jar from the request's `Cookie` headers. The parsed cookies are
    /// originals, so they are not sent back in the response.
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
