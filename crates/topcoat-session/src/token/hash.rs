use std::ops::Deref;

/// The SHA-256 hash of a [`Token`](crate::Token), identifying a session.
///
/// Store this value as the session's lookup key. A hash cannot be presented
/// as a session token. Obtain it from [`start`](crate::start) for a new session
/// or [`token_hash`](crate::token_hash) for the current request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    /// Creates a hash from raw bytes, typically loaded back out of the
    /// application's session storage.
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
