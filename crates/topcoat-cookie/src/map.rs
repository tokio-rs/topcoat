use cookie::Cookie;

use crate::Cookies;

/// A [`Cookies`] adapter that applies a transform to every cookie written
/// through it.
///
/// Created with [`Cookies::map`]. The closure changes a cookie before it is
/// passed to the wrapped jar.
///
/// The transform runs on `add` and `remove` alike, so `Path`/`Domain` defaults
/// reach the removal cookie and the browser can match it. `get` delegates
/// unchanged.
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
        let mut cookie = cookie.into();
        (self.f)(&mut cookie);
        self.inner.remove(cookie);
    }
}
