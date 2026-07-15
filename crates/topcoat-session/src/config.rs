use topcoat_core::context::{Cx, app_context};

use crate::SessionTokenStore;

pub struct SessionConfig {
    token_store: Box<dyn SessionTokenStore>,
}

pub fn session_config(cx: &Cx) -> &SessionConfig {
    app_context(cx)
}
