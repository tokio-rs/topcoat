use std::time::Duration;

use topcoat_core::context::{Cx, try_context};

/// How long a session lives without being refreshed.
///
/// The lifetime sets the token's time to live and the expiry returned for the
/// application to store. Add it to the router's context to change the
/// lifetime. Sessions use [`SessionLifetime::DEFAULT`] otherwise.
///
/// ```rust
/// use std::time::Duration;
///
/// use topcoat::{router::Router, session::SessionLifetime};
///
/// let router = Router::builder()
///     .with(SessionLifetime(Duration::from_hours(24 * 14)))
///     .build();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLifetime(pub Duration);

impl SessionLifetime {
    /// The lifetime of 30 days used when the context has none.
    pub const DEFAULT: Self = Self(Duration::from_hours(24 * 30));
}

impl Default for SessionLifetime {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub(crate) fn lifetime(cx: &Cx) -> Duration {
    try_context::<SessionLifetime>(cx)
        .copied()
        .unwrap_or_default()
        .0
}
