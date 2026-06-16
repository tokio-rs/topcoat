use cookie::Cookie;

use crate::Cookies;

/// A [`Cookies`] adapter that applies a transform to every cookie written
/// through it.
///
/// `Map` is how the attribute combinators ([`Cookies::default_same_site`],
/// [`Cookies::override_secure`], …) and the general [`Cookies::map`] escape
/// hatch are implemented: the closure runs on `add`, mutating the cookie before
/// it is forwarded inward.
///
/// The transform applies to writes only — `get` delegates unchanged (incoming
/// requests carry no attributes), and `remove` is left untouched so a removal
/// cookie's `Max-Age=0` is never clobbered.
#[derive(Debug, Clone, Copy)]
pub struct Map<J, F> {
    inner: J,
    f: F,
}

impl<J, F> Map<J, F> {
    pub(crate) fn new(inner: J, f: F) -> Self {
        Self { inner, f }
    }
}

impl<J, F> Cookies for Map<J, F>
where
    J: Cookies,
    F: Fn(&mut Cookie<'static>),
{
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.inner.get(name)
    }

    fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let mut cookie = cookie.into();
        (self.f)(&mut cookie);
        self.inner.add(cookie);
    }

    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
        self.inner.remove(cookie);
    }
}
