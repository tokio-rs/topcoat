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
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn random() -> Self {
        Self::new(rand::random())
    }

    #[must_use]
    pub fn decode(s: &str) -> Result<Self, DecodeError> {
        use base64::{DecodeSliceError, Engine as _, engine::general_purpose::URL_SAFE};
        let mut bytes = [0u8; 32];
        let num_bytes = URL_SAFE
            .decode_slice(s, &mut bytes)
            .map_err(|error| match error {
                DecodeSliceError::OutputSliceTooSmall => DecodeError::Length,
                DecodeSliceError::DecodeError(error) => error.into(),
            })?;
        if num_bytes != bytes.len() {
            return Err(DecodeError::Length);
        }
        Ok(Self::new(bytes))
    }

    #[must_use]
    pub(crate) fn encode(&self) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE};
        URL_SAFE.encode(self.0)
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

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("base64 decoding failed")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid number of bytes in token")]
    Length,
}

#[memoize]
pub fn token(cx: &Cx) -> Option<Token> {
    token_store(cx).read(cx)
}
