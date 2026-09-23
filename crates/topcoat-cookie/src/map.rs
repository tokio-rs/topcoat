use cookie::Cookie;

use crate::Cookies;

/// A [`Cookies`] adapter that runs a closure on every cookie written through
/// it.
///
/// The closure runs on both `add` and `remove`, so attributes such as `Path`
/// and `Domain` also reach removal cookies and the browser can match them.
/// Reads pass through unchanged.
///
/// Created by [`Cookies::map`] and the attribute combinators, such as
/// [`Cookies::default_same_site`].
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
