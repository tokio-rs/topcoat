use cookie::Cookie;

use crate::Cookies;

/// A [RFC 6265bis] cookie name prefix.
///
/// A prefix asks the browser to enforce extra constraints on a cookie based on
/// its name. Apply one with [`Cookies::override_prefix_host`] and friends.
///
/// [RFC 6265bis]: https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis#name-cookie-name-prefixes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prefix {
    /// The `__Host-` prefix: the cookie must be `Secure`, have `Path=/`, and no
    /// `Domain`.
    Host,
    /// The `__Secure-` prefix: the cookie must be `Secure`.
    Secure,
}

impl Prefix {
    /// The literal prefix string prepended to cookie names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Prefix::Host => "__Host-",
            Prefix::Secure => "__Secure-",
        }
    }

    /// Prepends the prefix to the cookie's name, unless it is already present.
    fn apply_name(self, cookie: &mut Cookie<'static>) {
        if !cookie.name().starts_with(self.as_str()) {
            let name = format!("{}{}", self.as_str(), cookie.name());
            cookie.set_name(name);
        }
    }

    /// Strips the prefix from the cookie's name, if present.
    fn strip_name(self, cookie: &mut Cookie<'static>) {
        if let Some(stripped) = cookie.name().strip_prefix(self.as_str()) {
            let stripped = stripped.to_owned();
            cookie.set_name(stripped);
        }
    }

    /// Applies the attributes the prefix requires.
    ///
    /// With [`Conform::Override`] the required attributes are forced (RFC
    /// compliant). With [`Conform::Default`] they are only filled in when the
    /// cookie does not already set them, leaving any explicit caller value
    /// untouched.
    fn conform(self, cookie: &mut Cookie<'static>, mode: Conform) {
        match (self, mode) {
            (Prefix::Host, Conform::Override) => {
                cookie.set_secure(true);
                cookie.set_path("/");
                cookie.unset_domain();
            }
            (Prefix::Host, Conform::Default) => {
                if cookie.secure().is_none() {
                    cookie.set_secure(true);
                }
                if cookie.path().is_none() {
                    cookie.set_path("/");
                }
            }
            (Prefix::Secure, Conform::Override) => {
                cookie.set_secure(true);
            }
            (Prefix::Secure, Conform::Default) => {
                if cookie.secure().is_none() {
                    cookie.set_secure(true);
                }
            }
        }
    }
}

/// Whether a prefix forces its required attributes or only fills them when
/// unset. See [`Prefix::conform`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum Conform {
    /// Fill required attributes only when the cookie has not set them.
    Default,
    /// Force the required attributes, guaranteeing RFC compliance.
    Override,
}

/// A [`Cookies`] adapter that scopes cookies to a name [`Prefix`].
///
/// On write it prepends the prefix and applies the prefix's required
/// attributes; on read it looks the cookie up under its prefixed name and
/// strips the prefix from the result. Created by
/// [`Cookies::override_prefix_host`] and the related combinators.
#[derive(Debug, Clone, Copy)]
pub struct Prefixed<J> {
    inner: J,
    prefix: Prefix,
    conform: Conform,
}

impl<J> Prefixed<J> {
    pub(crate) fn new(inner: J, prefix: Prefix, conform: Conform) -> Self {
        Self {
            inner,
            prefix,
            conform,
        }
    }
}

impl<J: Cookies> Cookies for Prefixed<J> {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        let prefixed = format!("{}{}", self.prefix.as_str(), name);
        let mut cookie = self.inner.get(&prefixed)?;
        self.prefix.strip_name(&mut cookie);
        Some(cookie)
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let mut cookie = cookie.into();
        self.prefix.apply_name(&mut cookie);
        self.prefix.conform(&mut cookie, self.conform);
        self.inner.add(cookie);
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let mut cookie = cookie.into();
        self.prefix.apply_name(&mut cookie);
        // A removal cookie must carry the prefix's required attributes (notably
        // `Path=/` for `__Host-`) so the browser matches and clears it.
        self.prefix.conform(&mut cookie, Conform::Override);
        self.inner.remove(cookie);
    }
}
