use std::fmt;

/// Error type for the prototype: a plain message.
///
/// The real framework error preserves the concrete type for downcasting; the
/// prototype only needs errors to travel, so a message is enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn msg(msg: impl Into<String>) -> Self {
        Error(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Result alias matching the shape component code uses.
pub type Result<T = (), E = Error> = std::result::Result<T, E>;
