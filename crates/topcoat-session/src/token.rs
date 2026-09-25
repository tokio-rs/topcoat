mod hash;
mod store;

pub use hash::*;
use sha2::Digest;
pub use store::*;

/// A 32-byte credential identifying a session.
///
/// Generate tokens with [`random`](Self::random). Send the token to the client
/// through a [`TokenStore`] and persist only its [`hash`](Self::hash).
#[derive(Clone)]
pub struct Token([u8; 32]);

impl Token {
    /// Creates a token from raw bytes, typically inside a [`TokenStore`] that
    /// has deserialized a client-presented token.
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generates a fresh random token.
    ///
    /// # Panics
    ///
    /// Panics when the platform's source of cryptographically secure
    /// randomness fails.
    #[must_use]
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the session token randomness source failed");
        Self::new(bytes)
    }

    /// Parses the URL-safe base64 form produced by [`encode`](Self::encode).
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] when `s` is not valid base64 or does not
    /// decode to exactly 32 bytes.
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

    /// Encodes the token as URL-safe base64.
    #[must_use]
    pub fn encode(&self) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE};
        URL_SAFE.encode(self.0)
    }

    /// Returns the [`TokenHash`] identifying this token's session, safe for
    /// the application to persist.
    #[must_use]
    pub fn hash(&self) -> TokenHash {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.0);
        TokenHash::new(hasher.finalize().0)
    }

    /// Exposes the raw token bytes.
    ///
    /// Only a [`TokenStore`] serializing the token for the client should need
    /// this; never persist the raw bytes server-side.
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

/// The reason a [`Token::decode`] call rejected its input.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("base64 decoding failed")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid number of bytes in token")]
    Length,
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE};

    use super::*;

    #[test]
    fn encode_then_decode_round_trips() {
        let token = Token::random();
        let decoded = Token::decode(&token.encode()).expect("an encoded token decodes back");
        assert_eq!(decoded.dangerous_as_array(), token.dangerous_as_array());
    }

    #[test]
    fn decode_accept_correct_length() {
        let encoded = URL_SAFE.encode([0u8; 32]);
        assert_eq!(
            Token::decode(&encoded).unwrap().dangerous_as_array(),
            &[0u8; 32]
        );
    }

    #[test]
    fn decode_rejects_length_too_short() {
        let encoded = URL_SAFE.encode([0u8; 31]);
        assert!(matches!(Token::decode(&encoded), Err(DecodeError::Length)));
    }

    #[test]
    fn decode_rejects_length_too_long() {
        let encoded = URL_SAFE.encode([0u8; 33]);
        assert!(matches!(Token::decode(&encoded), Err(DecodeError::Length)));
    }

    #[test]
    fn decode_rejects_invalid_base64() {
        assert!(matches!(
            Token::decode("not*valid*base64"),
            Err(DecodeError::Base64(_))
        ));
    }
}
