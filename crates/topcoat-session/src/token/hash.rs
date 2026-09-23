use std::ops::Deref;

/// The SHA-256 hash of a [`Token`](crate::Token), which identifies a session.
///
/// The application saves this value and looks sessions up by it. The token
/// cannot be recovered from the hash, so a leaked session database contains
/// nothing a client could use to log in. Get one from [`start`](crate::start)
/// when creating a session, or from [`token_hash`](crate::token_hash) for the
/// current request.
///
/// It dereferences to the 32 hash bytes, for saving in a database.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    /// Creates a hash from raw bytes, for example ones loaded from the
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
