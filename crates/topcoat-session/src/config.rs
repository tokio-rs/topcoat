use std::time::Duration;

use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

/// The token transport and lifetime for sessions.
///
/// Register it with the router's `sessions` method. Use
/// [`SessionConfig::builder`] to customize it, or `SessionConfig::default()`
/// to use the default cookie transport and lifetime.
pub struct SessionConfig {
    pub(crate) token_store: Box<dyn TokenStore>,
    pub(crate) lifetime: Duration,
}

/// How long a session lives without being refreshed, unless overridden with
/// [`SessionConfigBuilder::lifetime`]: 30 days.
pub const DEFAULT_LIFETIME: Duration = Duration::from_hours(24 * 30);

impl SessionConfig {
    /// Creates a builder for a session configuration.
    #[must_use]
    pub fn builder() -> SessionConfigBuilder {
        SessionConfigBuilder::default()
    }
}

/// Builds the all-defaults configuration, like [`SessionConfig::builder`] with an
/// immediate [`build`](SessionConfigBuilder::build).
#[cfg(feature = "cookie")]
impl Default for SessionConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Assembles a [`SessionConfig`]. Created with [`SessionConfig::builder`].
pub struct SessionConfigBuilder {
    token_store: Option<Box<dyn TokenStore>>,
    lifetime: Duration,
}

impl SessionConfigBuilder {
    /// Overrides the [`TokenStore`] carrying the session token between the
    /// client and the server.
    #[must_use]
    pub fn token_store(mut self, token_store: impl TokenStore + 'static) -> Self {
        self.token_store = Some(Box::new(token_store));
        self
    }

    /// Overrides how long a session lives without being refreshed.
    ///
    /// This sets the token's time to live and the expiry returned for the
    /// application to store.
    #[must_use]
    pub fn lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Consumes the builder, returning the finished [`SessionConfig`].
    ///
    /// # Panics
    ///
    /// Panics when no token store was set and the default cookie store is
    /// unavailable because the `cookie` feature is disabled.
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
