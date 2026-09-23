use cookie::{Cookie, CookieJar as RawCookieJar, Key};

use crate::Cookies;

/// A [`Cookies`] adapter that encrypts cookies on write and decrypts them on
/// read, using a [`Key`].
///
/// Encryption with AES-256-GCM makes a cookie's value both tamper-proof and
/// unreadable by the client. Reads return `None` when the cookie is missing or
/// fails to decrypt.
///
/// The encryption also covers the cookie's name, so a value cannot be moved to
/// a cookie with a different name. This works with any combination of adapters,
/// as long as the cookie is read through the same adapters it was written
/// through.
///
/// Created by [`Cookies::private`] or [`private_cookies`](crate::private_cookies).
#[derive(Debug, Clone, Copy)]
pub struct PrivateJar<'key, J> {
    inner: J,
    key: &'key Key,
}

impl<'key, J> PrivateJar<'key, J> {
    pub(crate) fn new(inner: J, key: &'key Key) -> Self {
        Self { inner, key }
    }
}

impl<J: Cookies> Cookies for PrivateJar<'_, J> {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        let raw = self.inner.get(name)?;
        // Decrypt using the cookie's own name as associated data, via a
        // throwaway jar seeded with the raw cookie as an original.
        let name = raw.name().to_owned();
        let mut jar = RawCookieJar::new();
        jar.add_original(raw);
        jar.private(self.key).get(&name)
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let cookie = cookie.into();
        let name = cookie.name().to_owned();
        let mut jar = RawCookieJar::new();
        jar.private_mut(self.key).add(cookie);
        if let Some(sealed) = jar.get(&name).cloned() {
            self.inner.add(sealed);
        }
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.inner.remove(cookie);
    }
}
