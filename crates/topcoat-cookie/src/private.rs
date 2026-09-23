use cookie::{Cookie, CookieJar as RawCookieJar, Key};

use crate::Cookies;

/// A [`Cookies`] adapter that encrypts cookies written through it and decrypts
/// cookies read through it, using a [`Key`].
///
/// Encryption (AES-256-GCM) makes a cookie's value both tamper-proof and
/// unreadable by the client. Created by [`Cookies::private`] or
/// [`private_cookies`](crate::private_cookies).
///
/// The cookie's **name is bound into the ciphertext as associated data**, so
/// the name seen at this layer must match on write and read. Because `get`
/// mirrors `add` through the same adapter stack, that holds automatically
/// regardless of how this layer is composed with prefixing. Reads return `None`
/// when a cookie is missing or fails to decrypt.
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
