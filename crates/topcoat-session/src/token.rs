mod hash;
mod store;

pub use hash::*;
pub use store::*;

use sha2::Digest;

/// A session token: 32 bytes of cryptographically secure randomness, held by
/// the client as its proof of a session.
///
/// The raw token only ever travels between the client and the [`TokenStore`].
/// Applications persist its [`hash`](Self::hash) instead, so a leaked session
/// database never contains a credential a client could present.
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
    #[must_use]
    pub fn random() -> Self {
        Self::new(rand::random())
    }

    /// Parses a token from its URL-safe base64 [`encode`](Self::encode)d form.
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] when `s` is not valid base64 or does not
    /// decode to exactly 32 bytes.
    pub fn decode(s: &str) -> Result<Self, DecodeError> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE};
        let bytes = URL_SAFE.decode(s)?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| DecodeError::Length)?;
        Ok(Self::new(bytes))
    }

    /// Encodes the token as URL-safe base64, for a [`TokenStore`] to send to
    /// the client.
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
    fn decode_rejects_wrong_length() {
        // A valid base64 string that decodes to 16 bytes, not the required 32.
        let encoded = URL_SAFE.encode([0u8; 16]);
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
