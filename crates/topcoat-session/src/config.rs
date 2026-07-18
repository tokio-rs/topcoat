use std::time::Duration;

use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

/// Session configuration, registered on the app context (with the router's
/// `sessions` extension method).
///
/// Assemble one with [`Config::builder`]; `Config::default()` is the
/// all-defaults configuration, carrying the token in the default cookie
/// store.
pub struct Config {
    pub(crate) token_store: Box<dyn TokenStore>,
    pub(crate) lifetime: Duration,
    #[cfg(feature = "router")]
    pub(crate) verify_origin: bool,
    #[cfg(feature = "router")]
    pub(crate) trusted_origins: Vec<String>,
}

/// How long a session lives without being refreshed, unless overridden with
/// [`ConfigBuilder::lifetime`]: 30 days.
pub const DEFAULT_LIFETIME: Duration = Duration::from_hours(24 * 30);

impl Config {
    /// Creates a builder for a session configuration.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

/// Builds the all-defaults configuration, like [`Config::builder`] with an
/// immediate [`build`](ConfigBuilder::build).
#[cfg(feature = "cookie")]
#[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
impl Default for Config {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Assembles a [`Config`]. Created with [`Config::builder`].
pub struct ConfigBuilder {
    token_store: Option<Box<dyn TokenStore>>,
    lifetime: Duration,
    #[cfg(feature = "router")]
    verify_origin: bool,
    #[cfg(feature = "router")]
    trusted_origins: Vec<String>,
}

impl ConfigBuilder {
    /// Overrides the [`TokenStore`] carrying the session token between the
    /// client and the server.
    #[must_use]
    pub fn token_store(mut self, token_store: impl TokenStore + 'static) -> Self {
        self.token_store = Some(Box::new(token_store));
        self
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

    /// Trusts `origin` to send state-changing cross-origin requests, exempting
    /// it from [`verify_origin`](crate::verify_origin).
    ///
    /// The value is compared against the request's `Origin` header, so pass
    /// the full serialized origin: scheme, host, and any non-default port
    /// (`"https://accounts.example.com"`), with no trailing slash.
    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    #[must_use]
    pub fn trust_origin(mut self, origin: impl Into<String>) -> Self {
        self.trusted_origins.push(origin.into());
        self
    }

    /// Disables the [`OriginLayer`](crate::OriginLayer) that the router's
    /// `sessions` extension method registers.
    ///
    /// Without the layer, nothing rejects state-changing cross-origin
    /// requests; only disable it if the application enforces its own defense
    /// against cross-site request forgery.
    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    #[must_use]
    pub fn dangerous_disable_origin_verification(mut self) -> Self {
        self.verify_origin = false;
        self
    }

    /// Consumes the builder, returning the finished [`Config`].
    ///
    /// # Panics
    ///
    /// Panics when no token store was set and the default cookie store is
    /// unavailable because the `cookie` feature is disabled.
    #[must_use]
    pub fn build(self) -> Config {
        Config {
            token_store: self.token_store.unwrap_or_else(default_token_store),
            lifetime: self.lifetime,
            #[cfg(feature = "router")]
            verify_origin: self.verify_origin,
            #[cfg(feature = "router")]
            trusted_origins: self.trusted_origins,
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            token_store: None,
            lifetime: DEFAULT_LIFETIME,
            #[cfg(feature = "router")]
            verify_origin: true,
            #[cfg(feature = "router")]
            trusted_origins: Vec::new(),
        }
    }
}

#[cfg(feature = "cookie")]
fn default_token_store() -> Box<dyn TokenStore> {
    Box::new(crate::cookie::CookieTokenStore::new())
}

#[cfg(not(feature = "cookie"))]
fn default_token_store() -> Box<dyn TokenStore> {
    panic!(
        "no token store configured: set one with `ConfigBuilder::token_store` or enable the `cookie` feature for the default cookie store"
    )
}

pub(crate) fn config(cx: &Cx) -> &Config {
    app_context(cx)
}
