use topcoat_core::context::Cx;

use crate::{Token, config};

pub trait TokenStore: Send + Sync {
    fn get(&self, cx: &Cx) -> Option<Token>;
    fn set(&self, cx: &Cx, token: Token);
}

pub fn token_store(cx: &Cx) -> &dyn TokenStore {
    &*config(cx).token_store
}

#[cfg(feature = "cookie")]
pub mod cookie {
    use topcoat_cookie::{Cookie, Cookies, SameSite};
    use topcoat_core::context::Cx;

    use crate::{Token, TokenStore};

    fn cookies(cx: &Cx) -> impl Cookies {
        topcoat_cookie::cookies(cx)
            .override_same_site(SameSite::Lax)
            .override_http_only(true)
            .override_secure(true)
            .override_path("/")
            .override_prefix_host()
    }

    pub const SESSION_COOKIE_NAME: &str = "session";

    pub struct CookieTokenStore {}

    impl TokenStore for CookieTokenStore {
        fn get(&self, cx: &Cx) -> Option<Token> {
            let cookie = cookies(cx).get(SESSION_COOKIE_NAME)?;
            Token::decode(cookie.value_trimmed()).ok()
        }

        fn set(&self, cx: &Cx, token: Token) {
            cookies(cx).add(Cookie::new(SESSION_COOKIE_NAME, token.encode()));
        }
    }
}
