mod jar;
mod macros;
mod map;
mod prefix;
mod private;
mod signed;

pub use jar::CookieJar;
pub use map::Map;
pub use prefix::{Prefix, Prefixed};
pub use private::PrivateJar;
pub use signed::SignedJar;

use prefix::Conform;

pub use cookie::{Cookie, Expiration, Key, SameSite, time};

use topcoat_core::runtime::context::{Cx, app_state};

/// A request-scoped cookie jar.
///
/// `Cookies` is implemented by the root [`CookieJar`] and by every adapter
/// ([`SignedJar`], [`PrivateJar`], [`Prefixed`], [`Map`]). The three core
/// methods read and write cookies; the combinators wrap the jar in further
/// adapters, in the style of [`Iterator`].
///
/// Bring this trait into scope to use the combinators.
pub trait Cookies {
    /// Returns the cookie named `name`, or `None` if it is absent (or, for
    /// signed/encrypted jars, fails to verify).
    fn get(&self, name: &str) -> Option<Cookie<'static>>;

    /// Adds `cookie`. It is serialized into a `Set-Cookie` response header once
    /// the handler returns.
    fn add<C: Into<Cookie<'static>>>(&self, cookie: C);

    /// Removes `cookie`. If the request carried an original cookie with the
    /// same name, an expiring removal cookie is sent. Pass the same `Path`/
    /// `Domain` the cookie was set with.
    fn remove<C: Into<Cookie<'static>>>(&self, cookie: C);

    /// Wraps this jar so signed cookies are written and verified with `key`.
    fn signed(self, key: &Key) -> SignedJar<'_, Self>
    where
        Self: Sized,
    {
        SignedJar::new(self, key)
    }

    /// Wraps this jar so cookies are encrypted and decrypted with `key`.
    fn private(self, key: &Key) -> PrivateJar<'_, Self>
    where
        Self: Sized,
    {
        PrivateJar::new(self, key)
    }

    /// Wraps this jar so every added cookie is passed through `f` before being
    /// stored. The general escape hatch behind the attribute combinators.
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(&mut Cookie<'static>),
    {
        Map::new(self, f)
    }

    /// Forces the `__Host-` prefix and its required attributes (`Secure`,
    /// `Path=/`, no `Domain`) on every added cookie.
    fn override_prefix_host(self) -> Prefixed<Self>
    where
        Self: Sized,
    {
        Prefixed::new(self, Prefix::Host, Conform::Override)
    }

    /// Applies the `__Host-` prefix, filling its required attributes only when
    /// the cookie does not already set them.
    fn default_prefix_host(self) -> Prefixed<Self>
    where
        Self: Sized,
    {
        Prefixed::new(self, Prefix::Host, Conform::Default)
    }

    /// Forces the `__Secure-` prefix and its required `Secure` attribute on
    /// every added cookie.
    fn override_prefix_secure(self) -> Prefixed<Self>
    where
        Self: Sized,
    {
        Prefixed::new(self, Prefix::Secure, Conform::Override)
    }

    /// Applies the `__Secure-` prefix, setting `Secure` only when the cookie
    /// does not already set it.
    fn default_prefix_secure(self) -> Prefixed<Self>
    where
        Self: Sized,
    {
        Prefixed::new(self, Prefix::Secure, Conform::Default)
    }

    /// Sets `Secure` on every added cookie, overriding any existing value.
    fn override_secure(self, value: bool) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| cookie.set_secure(value))
    }

    /// Sets `Secure` on added cookies that do not already specify it.
    fn default_secure(self, value: bool) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| {
            if cookie.secure().is_none() {
                cookie.set_secure(value);
            }
        })
    }

    /// Sets `HttpOnly` on every added cookie, overriding any existing value.
    fn override_http_only(self, value: bool) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| cookie.set_http_only(value))
    }

    /// Sets `HttpOnly` on added cookies that do not already specify it.
    fn default_http_only(self, value: bool) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| {
            if cookie.http_only().is_none() {
                cookie.set_http_only(value);
            }
        })
    }

    /// Sets `SameSite` on every added cookie, overriding any existing value.
    fn override_same_site(self, value: SameSite) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| cookie.set_same_site(value))
    }

    /// Sets `SameSite` on added cookies that do not already specify it.
    fn default_same_site(self, value: SameSite) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| {
            if cookie.same_site().is_none() {
                cookie.set_same_site(value);
            }
        })
    }

    /// Sets `Path` on every added cookie, overriding any existing value.
    fn override_path(self, value: impl Into<String>) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        let value = value.into();
        self.map(move |cookie| cookie.set_path(value.clone()))
    }

    /// Sets `Path` on added cookies that do not already specify it.
    fn default_path(self, value: impl Into<String>) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        let value = value.into();
        self.map(move |cookie| {
            if cookie.path().is_none() {
                cookie.set_path(value.clone());
            }
        })
    }

    /// Sets `Domain` on every added cookie, overriding any existing value.
    fn override_domain(self, value: impl Into<String>) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        let value = value.into();
        self.map(move |cookie| cookie.set_domain(value.clone()))
    }

    /// Sets `Domain` on added cookies that do not already specify it.
    fn default_domain(self, value: impl Into<String>) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        let value = value.into();
        self.map(move |cookie| {
            if cookie.domain().is_none() {
                cookie.set_domain(value.clone());
            }
        })
    }

    /// Sets `Max-Age` on every added cookie, overriding any existing value.
    fn override_max_age(self, value: time::Duration) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| cookie.set_max_age(value))
    }

    /// Sets `Max-Age` on added cookies that do not already specify it.
    fn default_max_age(self, value: time::Duration) -> Map<Self, impl Fn(&mut Cookie<'static>)>
    where
        Self: Sized,
    {
        self.map(move |cookie| {
            if cookie.max_age().is_none() {
                cookie.set_max_age(value);
            }
        })
    }
}

/// Builds the root jar from the request. A named `fn` (rather than a closure)
/// so its type is a stable marker shared by [`cookies`] and [`write_cookies`]:
/// the latter peeks the memoize cache under this exact marker.
fn parse_jar(cx: &Cx, (): ()) -> CookieJar {
    CookieJar::from_request(cx)
}

/// Returns the request's root [`CookieJar`], parsing the incoming `Cookie`
/// header on first access and memoizing it for the rest of the request.
///
/// Use the [`Cookies`] combinators to layer signing, encryption, prefixes, or
/// default attributes on top.
#[must_use]
pub fn cookies(cx: &Cx) -> &CookieJar {
    cx.cache().memoize(cx, (), (), parse_jar)
}

/// Returns the root jar wrapped in a [`SignedJar`], using the [`Key`]
/// registered as app state.
///
/// # Panics
///
/// Panics if no [`Key`] was registered with `Router::app_state`.
#[must_use]
pub fn signed_cookies(cx: &Cx) -> SignedJar<'_, &CookieJar> {
    cookies(cx).signed(app_state::<Key>(cx))
}

/// Returns the root jar wrapped in a [`PrivateJar`], using the [`Key`]
/// registered as app state.
///
/// # Panics
///
/// Panics if no [`Key`] was registered with `Router::app_state`.
#[must_use]
pub fn private_cookies(cx: &Cx) -> PrivateJar<'_, &CookieJar> {
    cookies(cx).private(app_state::<Key>(cx))
}

/// Appends the request's pending cookie changes to `headers` as `Set-Cookie`
/// entries.
///
/// Called by the router after each handler runs. If no cookie helper was used
/// during the request, the jar was never built — we skip without parsing the
/// incoming `Cookie` header at all.
#[doc(hidden)]
pub fn write_cookies(cx: &Cx, headers: &mut http::HeaderMap) {
    let Some(jar) = cx.cache().get::<_, CookieJar, _>(parse_jar, ()) else {
        return;
    };
    for value in jar.delta_headers() {
        headers.append(http::header::SET_COOKIE, value);
    }
}
