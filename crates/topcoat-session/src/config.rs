use std::time::Duration;

use topcoat_core::context::{Cx, app_context};

use crate::TokenStore;

/// Session configuration, registered on the app context (with the router's
/// `sessions` extension method).
pub struct Config {
    pub(crate) token_store: Box<dyn TokenStore>,
    pub(crate) lifetime: Duration,
    #[cfg(feature = "router")]
    pub(crate) verify_origin: bool,
    #[cfg(feature = "router")]
    pub(crate) trusted_origins: Vec<String>,
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
            #[cfg(feature = "router")]
            verify_origin: true,
            #[cfg(feature = "router")]
            trusted_origins: Vec::new(),
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

    /// Trusts `origin` to send state-changing cross-origin requests, exempting
    /// it from [`verify_origin`](crate::verify_origin).
    ///
    /// The value is compared against the request's `Origin` header, so pass
    /// the full serialized origin: scheme, host, and any non-default port
    /// (`"https://accounts.example.com"`), with no trailing slash.
    #[cfg(feature = "router")]
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
    #[must_use]
    pub fn dangerous_disable_origin_verification(mut self) -> Self {
        self.verify_origin = false;
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
