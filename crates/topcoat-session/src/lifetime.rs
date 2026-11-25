use std::time::Duration;

use topcoat_core::context::{Cx, try_context};

/// How long a session lives without being refreshed.
///
/// The lifetime sets the token's time to live and the expiry returned for the
/// application to store. Add it to the router's context to change the
/// lifetime. Sessions use [`SessionLifetime::DEFAULT`] otherwise.
///
/// Sessions round the lifetime up to whole seconds before using it for
/// [`Session::expires_at`](crate::Session::expires_at) and for the time to
/// live passed to [`TokenStore::write`](crate::TokenStore::write), so a token
/// store that keeps that time to live does not expire the token before the
/// record. A client may still discard the token sooner: browsers limit how
/// long a cookie may live.
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

    fn rounded_up_to_whole_seconds(self) -> Duration {
        let seconds = self
            .0
            .as_secs()
            .saturating_add(u64::from(self.0.subsec_nanos() > 0));
        Duration::from_secs(seconds)
    }
}

impl Default for SessionLifetime {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub(crate) fn time_to_live(cx: &Cx) -> Duration {
    try_context::<SessionLifetime>(cx)
        .copied()
        .unwrap_or_default()
        .rounded_up_to_whole_seconds()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rounded(lifetime: Duration) -> Duration {
        SessionLifetime(lifetime).rounded_up_to_whole_seconds()
    }

    #[test]
    fn whole_seconds_are_kept() {
        assert_eq!(rounded(Duration::ZERO), Duration::ZERO);
        assert_eq!(rounded(Duration::from_hours(2)), Duration::from_hours(2));
    }

    #[test]
    fn any_fraction_rounds_up() {
        assert_eq!(rounded(Duration::new(1, 1)), Duration::from_secs(2));
        assert_eq!(
            rounded(Duration::new(1, 999_999_999)),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn the_largest_lifetime_saturates() {
        assert_eq!(rounded(Duration::MAX), Duration::from_secs(u64::MAX));
    }
}
