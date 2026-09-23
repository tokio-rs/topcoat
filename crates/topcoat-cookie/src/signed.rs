use cookie::{Cookie, CookieJar as RawCookieJar, Key};

use crate::Cookies;

/// A [`Cookies`] adapter that signs cookies written through it and verifies
/// cookies read through it, using a [`Key`].
///
/// Signing makes a cookie tamper-proof while leaving its value readable by the
/// client. Created by [`Cookies::signed`] or [`signed_cookies`](crate::signed_cookies).
///
/// The signature is computed over the cookie's **value only**, so this layer
/// composes freely with prefixing and attribute defaults in any order. Reads
/// return `None` when a cookie is missing or its signature does not verify.
#[derive(Debug, Clone, Copy)]
pub struct SignedJar<'key, J> {
    inner: J,
    key: &'key Key,
}

impl<'key, J> SignedJar<'key, J> {
    pub(crate) fn new(inner: J, key: &'key Key) -> Self {
        Self { inner, key }
    }
}

impl<J: Cookies> Cookies for SignedJar<'_, J> {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        let raw = self.inner.get(name)?;
        // Reuse the cookie crate's verification by routing the raw cookie
        // through a throwaway jar seeded with it as an original.
        let name = raw.name().to_owned();
        let mut jar = RawCookieJar::new();
        jar.add_original(raw);
        jar.signed(self.key).get(&name)
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let cookie = cookie.into();
        // Sign via a throwaway jar, then forward the signed cookie inward so
        // any further layers (prefix, defaults) still apply.
        let name = cookie.name().to_owned();
        let mut jar = RawCookieJar::new();
        jar.signed_mut(self.key).add(cookie);
        if let Some(signed) = jar.get(&name).cloned() {
            self.inner.add(signed);
        }
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        // Removal cookies match by name only; the signature is irrelevant.
        self.inner.remove(cookie);
    }
}
