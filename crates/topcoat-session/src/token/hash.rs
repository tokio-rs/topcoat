use std::ops::Deref;

use topcoat_core::context::Cx;
use topcoat_core_macro::memoize;

use crate::{Token, token};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl Deref for TokenHash {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[memoize]
pub fn token_hash(cx: &Cx) -> Option<TokenHash> {
    token(cx).map(Token::hash)
}
