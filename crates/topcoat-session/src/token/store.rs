use std::{pin::Pin, time::Duration};

use topcoat_core::{context::Cx, error::Result};

use crate::{Token, config};

/// The boxed future returned by [`TokenStore`] methods.
pub type TokenStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Carries the session [`Token`] between the client and the server.
///
/// A token store is not the session database, which the application owns.
/// The default [`CookieTokenStore`](cookie::CookieTokenStore) carries the
/// token in a cookie. Implement this trait to carry it some other way, such
/// as in an `Authorization` header. Set it with
/// [`SessionConfigBuilder::token_store`](crate::SessionConfigBuilder::token_store).
pub trait TokenStore: Send + Sync {
    /// Reads the token the current request carries.
    ///
    /// Returns `None` when the request carries no token or an invalid one.
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>>;

    /// Sends `token` to the client, valid for `max_age`, replacing any token
    /// the client holds.
    fn write<'a>(&'a self, cx: &'a Cx, token: Token, max_age: Duration)
    -> TokenStoreFuture<'a, ()>;

    /// Tells the client to discard its token.
    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()>;
}

pub(crate) fn token_store(cx: &Cx) -> &dyn TokenStore {
    &*config(cx).token_store
}

/// The default token store, which carries the token in a cookie.
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

    /// A [`TokenStore`] that carries the token in a cookie that is `__Host-`
    /// prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, and scoped to `/`.
    ///
    /// Requires the cookie layer, which is added with `.cookies()` on the
    /// router builder.
    pub struct CookieTokenStore {
        name: Cow<'static, str>,
    }

    impl CookieTokenStore {
        /// Creates a store that uses [`SESSION_COOKIE_NAME`] as the cookie
        /// name.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Sets the name of the session cookie, without the `__Host-`
        /// prefix.
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
