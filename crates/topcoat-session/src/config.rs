use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

pub struct Config {
    pub(crate) token_store: Box<dyn TokenStore>,
}

pub fn config(cx: &Cx) -> &Config {
    app_context(cx)
}
