use std::ops::Deref;

/// The SHA-256 hash of a [`Token`](crate::Token), identifying a session.
///
/// This is the value the application persists (and looks sessions up by):
/// deriving the token from it is infeasible, so a leaked session database
/// contains nothing a client could present. Obtain one from
/// [`start`](crate::start) when creating a session, or
/// [`token_hash`](crate::token_hash) when resolving the current request's
/// session.
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
