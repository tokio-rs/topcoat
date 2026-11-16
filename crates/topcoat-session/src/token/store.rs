use std::{pin::Pin, time::Duration};

use topcoat_core::{
    context::{Cx, try_context},
    error::Result,
};

use crate::Token;

/// The future returned by [`TokenStore`] methods: a boxed, `Send` future
/// borrowing the store and the request context.
pub type TokenStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// The client-side transport for the session token.
///
/// Reads a raw [`Token`] from the request and sends token changes to the
/// client. The application stores session records separately. Implement this
/// trait to choose a transport other than the default cookie.
pub trait TokenStore: Send + Sync {
    /// Reads the token presented by the current request, or `None` when the
    /// request carries none (or a malformed one).
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>>;

    /// Issues `token` to the client with the given time to live, replacing
    /// any previously issued token.
    ///
    /// The time to live is the [`SessionLifetime`](crate::SessionLifetime)
    /// rounded up to whole seconds. Issue the token for at least that long, so
    /// the session record does not outlive it.
    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        time_to_live: Duration,
    ) -> TokenStoreFuture<'a, ()>;

    /// Instructs the client to discard its token.
    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()>;
}

/// The [`TokenStore`] that sessions use.
///
/// Add it to the router's context to replace the default store, which carries
/// the token in a cookie when the `cookie` feature is enabled and needs the
/// router's `CookieLayer`. Without that feature there is no default. A
/// session function panics when it needs the token store and the context has
/// no `SessionTokenStore`, and the default store is unavailable.
///
/// ```rust
/// use topcoat::{
///     router::Router,
///     session::{SessionTokenStore, cookie::CookieTokenStore},
/// };
///
/// let router = Router::builder()
///     .with(SessionTokenStore::new(CookieTokenStore::new().name("id")))
///     .build();
/// ```
pub struct SessionTokenStore(Box<dyn TokenStore>);

impl SessionTokenStore {
    /// Creates a session token store that uses `store`.
    #[must_use]
    pub fn new(store: impl TokenStore + 'static) -> Self {
        Self(Box::new(store))
    }
}

/// Returns the context's token store, or the default store when it has none.
///
/// # Panics
///
/// Panics when the context has no [`SessionTokenStore`] and either the
/// `cookie` feature is disabled or the router has no `CookieLayer`.
#[track_caller]
pub(crate) fn token_store(cx: &Cx) -> &dyn TokenStore {
    match try_context::<SessionTokenStore>(cx) {
        Some(store) => &*store.0,
        None => default_token_store(cx),
    }
}

#[cfg(feature = "cookie")]
#[track_caller]
fn default_token_store(cx: &Cx) -> &'static dyn TokenStore {
    static STORE: cookie::CookieTokenStore = cookie::CookieTokenStore::new();
    // Panic when the store is resolved rather than when it is first used, so
    // a missing cookie layer stops `prepare_start` before a record is stored.
    let _ = topcoat_cookie::cookies(cx);
    &STORE
}

#[cfg(not(feature = "cookie"))]
#[track_caller]
fn default_token_store(_cx: &Cx) -> &'static dyn TokenStore {
    panic!(
        "no session token store configured: add a `SessionTokenStore` to the context or enable \
         the `cookie` feature for the default cookie store"
    )
}

#[cfg(feature = "cookie")]
pub mod cookie {
    use std::{borrow::Cow, time::Duration};

    use topcoat_cookie::{Cookie, Cookies, SameSite};
    use topcoat_core::context::Cx;

    use crate::{Token, TokenStore, TokenStoreFuture};

    fn cookies(cx: &Cx) -> impl Cookies {
        topcoat_cookie::cookies(cx)
            .override_same_site(SameSite::Lax)
            .override_http_only(true)
            .override_secure(true)
            .override_path("/")
            .override_prefix_host()
    }

    /// The default name of the session cookie.
    pub const SESSION_COOKIE_NAME: &str = "session";

    /// A [`TokenStore`] carrying the token in a hardened cookie: `__Host-`
    /// prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, and scoped to `/`.
    ///
    /// Requires the router to have a `CookieLayer`.
    ///
    /// # Panics
    ///
    /// Every method panics, as any cookie access does, when the router has
    /// no `CookieLayer`. Writing or deleting the cookie also panics when the
    /// response headers have already been sent.
    pub struct CookieTokenStore {
        name: Cow<'static, str>,
    }

    impl CookieTokenStore {
        /// Creates a store using [`SESSION_COOKIE_NAME`].
        #[must_use]
        pub const fn new() -> Self {
            Self {
                name: Cow::Borrowed(SESSION_COOKIE_NAME),
            }
        }

        /// Overrides the name of the session cookie.
        #[must_use]
        pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
            self.name = name.into();
            self
        }
    }

    impl Default for CookieTokenStore {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TokenStore for CookieTokenStore {
        fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
            Box::pin(async move {
                let Some(cookie) = cookies(cx).get(&self.name) else {
                    return Ok(None);
                };
                Ok(Token::decode(cookie.value_trimmed()).ok())
            })
        }

        fn write<'a>(
            &'a self,
            cx: &'a Cx,
            token: Token,
            time_to_live: Duration,
        ) -> TokenStoreFuture<'a, ()> {
            Box::pin(async move {
                let cookies = cookies(cx);
                let max_age = topcoat_cookie::time::Duration::try_from(time_to_live)?;
                cookies
                    .override_max_age(max_age)
                    .add(Cookie::new(self.name.clone(), token.encode()));
                Ok(())
            })
        }

        fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
            Box::pin(async move {
                cookies(cx).remove(Cookie::new(self.name.clone(), ""));
                Ok(())
            })
        }
    }
}
