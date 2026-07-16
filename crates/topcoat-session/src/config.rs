use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

pub struct Config {
    pub(crate) token_store: Box<dyn TokenStore>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token_store: Box::new(crate::cookie::CookieTokenStore::default()),
        }
    }
}

pub fn config(cx: &Cx) -> &Config {
    app_context(cx)
}
