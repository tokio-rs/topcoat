use std::time::Duration;

use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

/// Session configuration: the [`TokenStore`] and the session lifetime.
///
/// Build one with [`SessionConfig::builder`]. With the `cookie` feature,
/// `SessionConfig::default()` returns the default configuration, which
/// carries the token in a [`CookieTokenStore`](crate::cookie::CookieTokenStore).
/// Register it on the router with `.sessions(config)`.
pub struct SessionConfig {
    pub(crate) token_store: Box<dyn TokenStore>,
    pub(crate) lifetime: Duration,
}

/// The default session lifetime: 30 days.
///
/// Change it with [`SessionConfigBuilder::lifetime`].
pub const DEFAULT_LIFETIME: Duration = Duration::from_hours(24 * 30);

impl SessionConfig {
    /// Creates a builder for a session configuration.
    #[must_use]
    pub fn builder() -> SessionConfigBuilder {
        SessionConfigBuilder::default()
    }
}

/// Returns the default configuration, the same as calling
/// [`build`](SessionConfigBuilder::build) on a new [`SessionConfig::builder`].
#[cfg(feature = "cookie")]
impl Default for SessionConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Builder for a [`SessionConfig`], created with [`SessionConfig::builder`].
pub struct SessionConfigBuilder {
    token_store: Option<Box<dyn TokenStore>>,
    lifetime: Duration,
}

impl SessionConfigBuilder {
    /// Sets the [`TokenStore`] that carries the session token between the
    /// client and the server.
    #[must_use]
    pub fn token_store(mut self, token_store: impl TokenStore + 'static) -> Self {
        self.token_store = Some(Box::new(token_store));
        self
    }

    /// Sets how long a session lives without being refreshed. Defaults to
    /// [`DEFAULT_LIFETIME`].
    ///
    /// The lifetime is the time to live of every token sent to the client, and
    /// the `expires_at` returned by [`start`](crate::start),
    /// [`refresh`](crate::refresh), and [`rotate`](crate::rotate) is the
    /// current time plus the lifetime.
    #[must_use]
    pub fn lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Returns the finished [`SessionConfig`].
    ///
    /// Uses a [`CookieTokenStore`](crate::cookie::CookieTokenStore) when no
    /// token store was set.
    ///
    /// # Panics
    ///
    /// Panics when no token store was set and the `cookie` feature, which
    /// provides the default cookie store, is disabled.
    #[must_use]
    #[track_caller]
    pub fn build(self) -> SessionConfig {
        SessionConfig {
            token_store: self.token_store.unwrap_or_else(default_token_store),
            lifetime: self.lifetime,
        }
    }
}

impl Default for SessionConfigBuilder {
    fn default() -> Self {
        Self {
            token_store: None,
            lifetime: DEFAULT_LIFETIME,
        }
    }
}

#[cfg(feature = "cookie")]
fn default_token_store() -> Box<dyn TokenStore> {
    Box::new(crate::cookie::CookieTokenStore::new())
}

#[cfg(not(feature = "cookie"))]
#[track_caller]
fn default_token_store() -> Box<dyn TokenStore> {
    panic!(
        "no token store configured: set one with `SessionConfigBuilder::token_store` or enable the `cookie` feature for the default cookie store"
    )
}

#[track_caller]
pub(crate) fn config(cx: &Cx) -> &SessionConfig {
    app_context(cx)
}
