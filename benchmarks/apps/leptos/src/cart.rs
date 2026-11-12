use std::fmt::{self, Display, Formatter};

use leptos::{prelude::*, wasm_bindgen::JsCast};
use serde::{Deserialize, Serialize};
use web_sys::HtmlDocument;

/// Name of the cookie that holds the number of items in the cart.
const CART_COOKIE: &str = "cart_count";

/// The largest count the cart cookie stores.
const CART_MAX: u32 = 999_999_999;

/// The number of items in the cart, kept in the `cart_count` cookie.
///
/// A missing or malformed cookie counts as an empty cart. Displays as the badge text, which caps at
/// `99+`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
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

    /// Reads the count from the cookies of the request being rendered.
    #[cfg(feature = "ssr")]
    pub fn from_request() -> Self {
        use axum_extra::extract::CookieJar;

        use_context::<axum::http::request::Parts>()
            .and_then(|parts| {
                let jar = CookieJar::from_headers(&parts.headers);
                Self::parse(jar.get(CART_COOKIE)?.value())
            })
            .unwrap_or_default()
    }

    /// Reads the count from the browser's cookies.
    pub fn from_document() -> Self {
        html_document()
            .cookie()
            .ok()
            .and_then(|cookies| Self::from_cookie_string(&cookies))
            .unwrap_or_default()
    }

    /// Finds the count in a `name=value; name=value` string, the form `document.cookie` has.
    ///
    /// The browser only exposes its cookies as this one string, so it is split here.
    fn from_cookie_string(cookies: &str) -> Option<Self> {
        cookies.split(';').find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name == CART_COOKIE).then(|| Self::parse(value))?
        })
    }

    /// Adds `quantity` items, capped at the largest count the cookie stores.
    pub fn add(self, quantity: u32) -> Self {
        Self(self.0.saturating_add(quantity).min(CART_MAX))
    }

    /// Writes the count to the browser's cookies.
    pub fn save(self) {
        let cookie = format!(
            "{CART_COOKIE}={}; Path=/; Max-Age=2592000; SameSite=Lax",
            self.0
        );
        // Setting `document.cookie` only fails in sandboxed documents.
        let _ = html_document().set_cookie(&cookie);
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

fn html_document() -> HtmlDocument {
    document().unchecked_into()
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
    fn finds_the_cart_cookie_among_others() {
        let cookies = "theme=dark; cart_count=007; other=1";
        assert_eq!(CartCount::from_cookie_string(cookies), Some(CartCount(7)));
        assert_eq!(CartCount::from_cookie_string("cart_count=lots"), None);
        assert_eq!(CartCount::from_cookie_string("my_cart_count=4"), None);
    }

    #[test]
    fn badge_caps_large_counts() {
        assert_eq!(CartCount(100).to_string(), "99+");
        assert_eq!(CartCount(999_999_990).add(20), CartCount(999_999_999));
    }
}
