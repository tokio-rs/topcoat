mod hash;
mod store;

pub use hash::*;
use sha2::Digest;
pub use store::*;

/// A session token: 32 random bytes that the client holds as proof of its
/// session.
///
/// The token only travels between the client and the [`TokenStore`].
/// Applications save its [`hash`](Self::hash) instead, so a leaked session
/// database contains nothing a client could use to log in. The `Debug` output
/// does not show the bytes.
#[derive(Clone)]
pub struct Token([u8; 32]);

impl Token {
    /// Creates a token from raw bytes, for example in a [`TokenStore`] that
    /// reads the token from the client in its own format.
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Creates a new random token.
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

    /// Parses a token from the URL-safe base64 text returned by
    /// [`encode`](Self::encode).
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

    /// Encodes the token as URL-safe base64 text, for a [`TokenStore`] to send
    /// to the client.
    #[must_use]
    pub fn encode(&self) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE};
        URL_SAFE.encode(self.0)
    }

    /// Returns the SHA-256 [`TokenHash`] of this token, which the application
    /// can safely save.
    #[must_use]
    pub fn hash(&self) -> TokenHash {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.0);
        TokenHash::new(hasher.finalize().0)
    }

    /// Returns the raw token bytes.
    ///
    /// Only a [`TokenStore`] that sends the token to the client in its own
    /// format should need this. Never save the raw bytes on the server.
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

/// The error returned by [`Token::decode`] for invalid input.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The input is not valid URL-safe base64.
    #[error("base64 decoding failed")]
    Base64(#[from] base64::DecodeError),
    /// The input does not decode to exactly 32 bytes.
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
