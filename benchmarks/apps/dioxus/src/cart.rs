//! The cart badge count, kept in the `cart_count` cookie.

use std::fmt::{self, Display, Formatter};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// Name of the cookie that holds the number of items in the cart.
pub const CART_COOKIE: &str = "cart_count";

/// The number of items in the cart.
///
/// A missing or malformed cookie counts as an empty cart. Displays as the badge text, which caps at
/// `99+`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct CartCount(pub u32);

impl CartCount {
    /// The largest count the cookie stores.
    const MAX: u32 = 999_999_999;

    /// Parses a cookie value of 1 to 9 ASCII digits.
    pub fn parse(value: &str) -> Option<Self> {
        let valid = (1..=9).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit());
        if valid {
            value.parse().ok().map(Self)
        } else {
            None
        }
    }

    /// Adds `quantity` items, capped at the largest count the cookie stores.
    pub fn add(self, quantity: u32) -> Self {
        Self(self.0.saturating_add(quantity).min(Self::MAX))
    }

    /// Reads the count from the browser's cookies.
    #[cfg(feature = "web")]
    pub fn read_cookie() -> Self {
        let cookies = html_document()
            .and_then(|document| document.cookie().ok())
            .unwrap_or_default();
        cookies
            .split(';')
            .find_map(|cookie| cookie.trim_start().strip_prefix("cart_count="))
            .and_then(Self::parse)
            .unwrap_or_default()
    }

    /// Stores the count in the browser's cookies for 30 days.
    #[cfg(feature = "web")]
    pub fn write_cookie(self) {
        if let Some(document) = html_document() {
            let cookie = format!(
                "{CART_COOKIE}={}; Path=/; Max-Age=2592000; SameSite=Lax",
                self.0
            );
            _ = document.set_cookie(&cookie);
        }
    }

    /// Event handlers only run in the browser, so the server never reads the cookie this way.
    #[cfg(not(feature = "web"))]
    pub fn read_cookie() -> Self {
        Self::default()
    }

    /// Event handlers only run in the browser, so the server never writes the cookie this way.
    #[cfg(not(feature = "web"))]
    pub fn write_cookie(self) {}
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

/// Reads the cart count from the request's `cart_count` cookie.
#[server(headers: dioxus::fullstack::HeaderMap)]
pub async fn cart_count() -> ServerFnResult<CartCount> {
    let jar = axum_extra::extract::CookieJar::from_headers(&headers);
    let count = jar
        .get(CART_COOKIE)
        .and_then(|cookie| CartCount::parse(cookie.value()));
    Ok(count.unwrap_or_default())
}

#[cfg(feature = "web")]
fn html_document() -> Option<web_sys::HtmlDocument> {
    use wasm_bindgen::JsCast;

    web_sys::window()?.document()?.dyn_into().ok()
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

    #[test]
    fn adding_stops_at_the_cookie_limit() {
        assert_eq!(CartCount(999_999_998).add(5), CartCount(999_999_999));
    }
}
