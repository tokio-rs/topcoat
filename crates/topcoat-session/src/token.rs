use std::ops::Deref;

use sha2::Digest;
use topcoat_core::context::{Cx, app_context};
use topcoat_core_macro::memoize;

use crate::session_config;

#[derive(Clone)]
pub struct SessionToken([u8; 32]);

impl SessionToken {
    #[must_use]
    pub fn random() -> Self {
        Self(rand::random())
    }

    #[must_use]
    pub fn hash(&self) -> SessionTokenHash {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.0);
        SessionTokenHash(hasher.finalize().0)
    }

    #[must_use]
    pub fn dangerous_as_array(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionToken").finish()
    }
}

#[memoize]
pub fn session_token(cx: &Cx) -> Option<SessionToken> {
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionTokenHash([u8; 32]);

impl Deref for SessionTokenHash {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[memoize]
pub fn session_token_hash(cx: &Cx) -> Option<SessionTokenHash> {
    session_token(cx).map(SessionToken::hash)
}

pub trait SessionTokenStore: Send + Sync {
    fn get(&self, cx: &Cx) -> Option<SessionToken>;
    fn set(&self, cx: &Cx, token: SessionToken);
}

pub fn session_token_store(cx: &Cx) -> &dyn SessionTokenStore {
    session_config(cx)
}
