mod hash;
mod store;

pub use hash::*;
pub use store::*;

use sha2::Digest;
use topcoat_core::context::Cx;
use topcoat_core_macro::memoize;

#[derive(Clone)]
pub struct Token([u8; 32]);

impl Token {
    #[must_use]
    pub fn random() -> Self {
        Self(rand::random())
    }

    #[must_use]
    pub fn hash(&self) -> TokenHash {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.0);
        TokenHash::new(hasher.finalize().0)
    }

    #[must_use]
    pub fn dangerous_as_array(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Token)).finish()
    }
}

#[memoize]
pub fn token(cx: &Cx) -> Option<Token> {
    token_store(cx).get(cx)
}
