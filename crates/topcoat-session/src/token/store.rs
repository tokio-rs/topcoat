use topcoat_core::context::Cx;

use crate::{Token, config};

pub trait TokenStore: Send + Sync {
    fn get(&self, cx: &Cx) -> Option<Token>;
    fn set(&self, cx: &Cx, token: Token);
}

pub fn token_store(cx: &Cx) -> &dyn TokenStore {
    &*config(cx).token_store
}
