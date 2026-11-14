use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
};

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::CookieJar;

/// Name of the cookie that holds the number of items in the cart.
pub const CART_COOKIE: &str = "cart_count";

/// The number of items in the cart, read from the `cart_count` cookie.
///
/// A missing or malformed cookie counts as an empty cart. Displays as the badge text, which caps at
/// `99+`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CartCount(pub u32);

impl CartCount {
    /// Parses a cookie value of 1 to 9 ASCII digits.
    pub fn parse(value: &str) -> Option<Self> {
        let valid = (1..=9).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit());
        if valid {
            value.parse().ok().map(Self)
        } else {
            None
        }
    }
}

impl Display for CartCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 > 99 {
            f.write_str("99+")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CartCount {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let count = jar
            .get(CART_COOKIE)
            .and_then(|cookie| Self::parse(cookie.value()));
        Ok(count.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage() {
        for value in ["", "-1", "+1", "1.5", "abc", " 3", "1234567890"] {
            assert!(CartCount::parse(value).is_none(), "{value:?}");
        }
    }

    #[test]
    fn badge_caps_large_counts() {
        assert_eq!(CartCount::parse("007").unwrap().to_string(), "7");
        assert_eq!(CartCount(100).to_string(), "99+");
    }
}
