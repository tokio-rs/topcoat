mod jar;
mod macros;
mod map;
mod prefix;
mod private;
#[cfg(feature = "router")]
mod router;
mod signed;
mod store;

pub use jar::*;
pub use map::*;
pub use prefix::*;
pub use private::*;
#[cfg(feature = "router")]
pub use router::*;
pub use signed::*;
pub use store::*;

use prefix::Conform;
use std::sync::OnceLock;

pub use cookie::{Cookie, Expiration, Key, SameSite, time};

use topcoat_core::runtime::context::{Cx, app_context, request_context};

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
    /// the cookie router layer handles the response.
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

/// Request-context storage for the lazily built cookie jar.
///
/// The cookie router layer inserts one cell per request. The first call to
/// [`cookies`] parses the incoming `Cookie` headers into a [`CookieJar`] and
/// stores it here; response finalization reads the same cell to emit pending
/// `Set-Cookie` headers only if the jar was actually touched.
#[derive(Debug, Default)]
pub struct CookieJarCell {
    jar: OnceLock<CookieJar>,
}

impl CookieJarCell {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn get_or_init(&self, cx: &Cx) -> &CookieJar {
        self.jar.get_or_init(|| CookieJar::from_request(cx))
    }

    fn get(&self) -> Option<&CookieJar> {
        self.jar.get()
    }
}

/// Returns the request's root [`CookieJar`], parsing the incoming `Cookie`
/// header on first access and memoizing it for the rest of the request.
///
/// Use the [`Cookies`] combinators to layer signing, encryption, prefixes, or
/// default attributes on top.
///
/// # Panics
///
/// Panics if the cookie router layer has not been installed for this request.
#[must_use]
pub fn cookies(cx: &Cx) -> &CookieJar {
    request_context::<CookieJarCell>(cx).get_or_init(cx)
}

/// Returns the root jar wrapped in a [`SignedJar`], using the [`Key`]
/// registered as app context.
///
/// # Panics
///
/// Panics if no [`Key`] was registered with `Router::app_context`.
#[must_use]
pub fn signed_cookies(cx: &Cx) -> SignedJar<'_, &CookieJar> {
    cookies(cx).signed(app_context::<Key>(cx))
}

/// Returns the root jar wrapped in a [`PrivateJar`], using the [`Key`]
/// registered as app context.
///
/// # Panics
///
/// Panics if no [`Key`] was registered with `Router::app_context`.
#[must_use]
pub fn private_cookies(cx: &Cx) -> PrivateJar<'_, &CookieJar> {
    cookies(cx).private(app_context::<Key>(cx))
}

/// Appends the request's pending cookie changes to `headers` as `Set-Cookie`
/// entries.
///
/// Called by the router after each handler runs. If no cookie helper was used
/// during the request, the jar was never built, so we skip without parsing the
/// incoming `Cookie` header at all.
#[doc(hidden)]
pub fn write_cookies(cx: &Cx, headers: &mut http::HeaderMap) {
    let Some(jar) = request_context::<CookieJarCell>(cx).get() else {
        return;
    };
    for value in jar.delta_headers() {
        headers.append(http::header::SET_COOKIE, value);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::{HeaderMap, Request, header, request::Parts};
    use topcoat_core::runtime::context::ContextMap;

    use super::*;

    /// Builds a `Cx` whose request carries the given `Cookie` header values (one
    /// per entry, so multi-header parsing can be exercised). No `Parts` are
    /// registered when `cookie_headers` is empty *and* `with_parts` is false.
    fn cx_with(cookie_headers: &[&str]) -> Cx {
        let mut builder = Request::builder();
        for value in cookie_headers {
            builder = builder.header(header::COOKIE, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        let mut request_context = ContextMap::new();
        request_context.insert::<Parts>(parts);
        request_context.insert(CookieJarCell::new());
        Cx::new(Arc::new(ContextMap::new()), request_context)
    }

    /// Like [`cx_with`], but also registers `key` as app context so the
    /// `signed_cookies`/`private_cookies` helpers can find it.
    fn cx_with_key(cookie_headers: &[&str], key: Key) -> Cx {
        let mut builder = Request::builder();
        for value in cookie_headers {
            builder = builder.header(header::COOKIE, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        let mut request_context = ContextMap::new();
        request_context.insert::<Parts>(parts);
        request_context.insert(CookieJarCell::new());
        let mut app_context = ContextMap::new();
        app_context.insert::<Key>(key);
        Cx::new(Arc::new(app_context), request_context)
    }

    /// The `Set-Cookie` header values the request would emit.
    fn set_cookies(cx: &Cx) -> Vec<String> {
        let mut headers = HeaderMap::new();
        write_cookies(cx, &mut headers);
        headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect()
    }

    /// The leading `name=value` pair of a `Set-Cookie` value, i.e. what the
    /// browser would echo back in a `Cookie` header.
    fn pair(set_cookie: &str) -> &str {
        set_cookie.split(';').next().unwrap()
    }

    #[test]
    fn reads_incoming_cookies() {
        let cx = cx_with(&["theme=dark; lang=en"]);
        let jar = cookies(&cx);

        assert_eq!(jar.get("theme").unwrap().value(), "dark");
        assert_eq!(jar.get("lang").unwrap().value(), "en");
        assert!(jar.get("missing").is_none());
    }

    #[test]
    fn reads_cookies_across_multiple_headers() {
        let cx = cx_with(&["theme=dark", "lang=en"]);
        let jar = cookies(&cx);

        assert_eq!(jar.get("theme").unwrap().value(), "dark");
        assert_eq!(jar.get("lang").unwrap().value(), "en");
    }

    #[test]
    fn reading_does_not_produce_a_delta() {
        // Cookies that arrived on the request are "original" and must not be
        // echoed back as Set-Cookie just because they were read.
        let cx = cx_with(&["theme=dark"]);
        let _ = cookies(&cx).get("theme");

        assert!(set_cookies(&cx).is_empty());
    }

    #[test]
    fn add_emits_set_cookie() {
        let cx = cx_with(&[]);
        cookies(&cx).add(("theme", "dark"));

        let set = set_cookies(&cx);
        assert_eq!(set.len(), 1);
        assert_eq!(pair(&set[0]), "theme=dark");
    }

    #[test]
    fn remove_emits_expiring_cookie() {
        let cx = cx_with(&["session=abc123"]);
        cookies(&cx).remove(("session", ""));

        let set = set_cookies(&cx);
        assert_eq!(set.len(), 1);
        assert!(set[0].starts_with("session="));
        assert!(set[0].contains("Max-Age=0"), "{}", set[0]);
    }

    #[test]
    fn remove_replays_path_default_so_browser_matches() {
        // A cookie written with `default_path` must be removed with the same
        // Path, or the browser won't match and clear it.
        let cx = cx_with(&["session=abc123"]);
        cookies(&cx).default_path("/app").remove(("session", ""));

        let set = &set_cookies(&cx)[0];
        assert!(set.contains("Path=/app"), "{set}");
        assert!(set.contains("Max-Age=0"), "{set}");
    }

    #[test]
    fn remove_keeps_expiry_despite_max_age_default() {
        // A `default_max_age` must not leak onto a removal: the jar still expires
        // it with Max-Age=0.
        let cx = cx_with(&["session=abc123"]);
        cookies(&cx)
            .default_max_age(time::Duration::hours(1))
            .remove(("session", ""));

        assert!(
            set_cookies(&cx)[0].contains("Max-Age=0"),
            "{:?}",
            set_cookies(&cx)
        );
    }

    #[test]
    fn write_cookies_skips_when_jar_untouched() {
        // The jar is never accessed, so nothing should be written. Crucially,
        // we register no `Parts`: if `write_cookies` parsed the request anyway
        // it would panic looking them up, proving it short-circuits.
        let mut request_context = ContextMap::new();
        request_context.insert(CookieJarCell::new());
        let cx = Cx::new(Arc::new(ContextMap::new()), request_context);
        let mut headers = HeaderMap::new();
        write_cookies(&cx, &mut headers);

        assert!(headers.get(header::SET_COOKIE).is_none());
    }

    #[test]
    fn default_attribute_only_fills_when_unset() {
        let cx = cx_with(&[]);
        // Already has an explicit `Secure=false`, so the default is ignored.
        cookies(&cx)
            .default_secure(true)
            .add(Cookie::build(("a", "b")).secure(false).build());

        assert!(!set_cookies(&cx)[0].contains("Secure"));
    }

    #[test]
    fn override_attribute_replaces_existing() {
        let cx = cx_with(&[]);
        cookies(&cx)
            .override_secure(true)
            .add(Cookie::build(("a", "b")).secure(false).build());

        assert!(set_cookies(&cx)[0].contains("Secure"));
    }

    #[test]
    fn host_prefix_applies_name_and_attributes() {
        let cx = cx_with(&[]);
        cookies(&cx).override_prefix_host().add(("session", "abc"));

        let set = &set_cookies(&cx)[0];
        assert!(set.starts_with("__Host-session=abc"), "{set}");
        assert!(set.contains("Secure"), "{set}");
        assert!(set.contains("Path=/"), "{set}");
    }

    #[test]
    fn host_prefix_reads_back_under_bare_name() {
        // The browser sends the prefixed name; the adapter strips it on read.
        let cx = cx_with(&["__Host-session=abc"]);
        let cookie = cookies(&cx)
            .override_prefix_host()
            .get("session")
            .expect("prefixed cookie should be found");

        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.value(), "abc");
    }

    #[test]
    fn signed_cookie_round_trips() {
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookies(&writer).signed(&key).add(("user_id", "42"));
        let echoed = pair(&set_cookies(&writer)[0]).to_owned();

        let reader = cx_with(&[&echoed]);
        let cookie = cookies(&reader)
            .signed(&key)
            .get("user_id")
            .expect("valid signature should verify");
        assert_eq!(cookie.value(), "42");
    }

    #[test]
    fn signed_cookie_rejects_wrong_key_and_tampering() {
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookies(&writer).signed(&key).add(("user_id", "42"));
        let echoed = pair(&set_cookies(&writer)[0]).to_owned();

        // A different key cannot verify the signature.
        let reader = cx_with(&[&echoed]);
        assert!(
            cookies(&reader)
                .signed(&Key::generate())
                .get("user_id")
                .is_none()
        );

        // Neither can the correct key once the value is altered.
        let tampered = format!("{}x", &echoed[..echoed.len() - 1]);
        let reader = cx_with(&[&tampered]);
        assert!(cookies(&reader).signed(&key).get("user_id").is_none());
    }

    #[test]
    fn signed_value_stays_readable() {
        // Signing authenticates but does not hide the value.
        let key = Key::generate();
        let cx = cx_with(&[]);
        cookies(&cx).signed(&key).add(("user_id", "42"));

        assert!(set_cookies(&cx)[0].contains("42"));
    }

    #[test]
    fn private_cookie_round_trips_and_hides_value() {
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookies(&writer)
            .private(&key)
            .add(("session", "secret-token"));
        let echoed = pair(&set_cookies(&writer)[0]).to_owned();

        // Encryption hides the plaintext from the wire.
        assert!(!echoed.contains("secret-token"), "{echoed}");

        let reader = cx_with(&[&echoed]);
        let cookie = cookies(&reader)
            .private(&key)
            .get("session")
            .expect("correct key should decrypt");
        assert_eq!(cookie.value(), "secret-token");
    }

    #[test]
    fn private_cookie_rejects_wrong_key() {
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookies(&writer)
            .private(&key)
            .add(("session", "secret-token"));
        let echoed = pair(&set_cookies(&writer)[0]).to_owned();

        let reader = cx_with(&[&echoed]);
        assert!(
            cookies(&reader)
                .private(&Key::generate())
                .get("session")
                .is_none()
        );
    }

    #[test]
    fn signed_cookies_helper_uses_app_context_key() {
        let key = Key::generate();

        let writer = cx_with_key(&[], key.clone());
        signed_cookies(&writer).add(("user_id", "42"));
        let echoed = pair(&set_cookies(&writer)[0]).to_owned();

        let reader = cx_with_key(&[&echoed], key);
        assert_eq!(
            signed_cookies(&reader).get("user_id").unwrap().value(),
            "42"
        );
    }

    #[test]
    fn composes_signing_with_prefix_and_defaults() {
        // Layers stack in any order: the cookie is signed, prefixed, and gets a
        // default SameSite, and still verifies on read.
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookies(&writer)
            .signed(&key)
            .override_prefix_host()
            .default_same_site(SameSite::Lax)
            .add(("session", "abc"));

        let set = &set_cookies(&writer)[0];
        assert!(set.starts_with("__Host-session="), "{set}");
        assert!(set.contains("SameSite=Lax"), "{set}");
        let echoed = pair(set).to_owned();

        let reader = cx_with(&[&echoed]);
        let cookie = cookies(&reader)
            .signed(&key)
            .override_prefix_host()
            .get("session")
            .expect("composed layers should round-trip");
        assert_eq!(cookie.value(), "abc");
    }
}
