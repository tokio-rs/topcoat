use std::time::Duration;

use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

/// Session configuration, registered on the app context (with the router's
/// `sessions` extension method).
pub struct Config {
    pub(crate) token_store: Box<dyn TokenStore>,
    pub(crate) lifetime: Duration,
}

/// How long a session lives without being refreshed, unless overridden with
/// [`Config::lifetime`]: 30 days.
pub const DEFAULT_LIFETIME: Duration = Duration::from_hours(24 * 30);

impl Config {
    /// Creates a configuration carrying the session token with the given
    /// store.
    #[must_use]
    pub fn new(token_store: impl TokenStore + 'static) -> Self {
        Self {
            token_store: Box::new(token_store),
            lifetime: DEFAULT_LIFETIME,
        }
    }

    /// Overrides how long a session lives without being refreshed.
    ///
    /// The lifetime becomes the time to live of every issued token, and
    /// [`start`](crate::start), [`refresh`](crate::refresh), and
    /// [`rotate`](crate::rotate) derive the session's `expires_at` from it.
    #[must_use]
    pub fn lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime;
        self
    }
}

/// Carries the token in the default [`CookieTokenStore`](crate::cookie::CookieTokenStore).
#[cfg(feature = "cookie")]
impl Default for Config {
    fn default() -> Self {
        Self::new(crate::cookie::CookieTokenStore::new())
    }
}

pub(crate) fn config(cx: &Cx) -> &Config {
    app_context(cx)
}
