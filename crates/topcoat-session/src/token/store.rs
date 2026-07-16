use std::{pin::Pin, time::Duration};

use topcoat_core::{context::Cx, error::Result};

use crate::{Token, config};

/// The future returned by [`TokenStore`] methods: a boxed, `Send` future
/// borrowing the store and the request context.
pub type TokenStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// The client-side transport for the session token.
///
/// A token store moves the raw [`Token`] between the client and the server;
/// it is not the session database, which the application owns. The default
/// [`CookieTokenStore`](cookie::CookieTokenStore) carries the token in a
/// hardened cookie; implement this trait to carry it elsewhere, such as an
/// `Authorization` header.
pub trait TokenStore: Send + Sync {
    /// Reads the token presented by the current request, or `None` when the
    /// request carries none (or a malformed one).
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>>;

    /// Issues `token` to the client with the given time to live, replacing
    /// any previously issued token.
    fn write<'a>(&'a self, cx: &'a Cx, token: Token, max_age: Duration)
    -> TokenStoreFuture<'a, ()>;

    /// Instructs the client to discard its token.
    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()>;
}

pub(crate) fn token_store(cx: &Cx) -> &dyn TokenStore {
    &*config(cx).token_store
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
    /// Requires the cookie layer (`RouterBuilder::cookies`) to be registered
    /// on the router.
    pub struct CookieTokenStore {
        name: Cow<'static, str>,
    }

    impl CookieTokenStore {
        /// Creates a store using [`SESSION_COOKIE_NAME`].
        #[must_use]
        pub fn new() -> Self {
            Self::default()
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
            Self {
                name: Cow::Borrowed(SESSION_COOKIE_NAME),
            }
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
            max_age: Duration,
        ) -> TokenStoreFuture<'a, ()> {
            Box::pin(async move {
                let max_age = topcoat_cookie::time::Duration::try_from(max_age)?;
                cookies(cx)
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
