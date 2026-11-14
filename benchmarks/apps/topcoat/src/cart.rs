use topcoat::{
    context::Cx,
    cookie::{Cookies, cookies},
    runtime::{Signal, signal},
};

/// Name of the cookie that holds the number of items in the cart.
pub const CART_COOKIE: &str = "cart_count";

/// The largest number of items the cart counts.
pub const CART_MAX: u32 = 999_999_999;

/// The number of items in the cart, shared by the header badge and the add-to-cart button.
///
/// Every call site uses the same signal because the signal is declared here, below the request
/// context that layouts and pages share. The initial value comes from the `cart_count` cookie.
pub fn cart_signal(cx: &Cx) -> Signal<u32> {
    signal(cx, || cart_count(cx))
}

/// Reads the `cart_count` cookie. A missing or malformed cookie counts as an empty cart.
pub fn cart_count(cx: &Cx) -> u32 {
    cookies(cx)
        .get(CART_COOKIE)
        .and_then(|cookie| parse_cart_count(cookie.value()))
        .unwrap_or(0)
}

/// Parses a cookie value of 1 to 9 ASCII digits.
fn parse_cart_count(value: &str) -> Option<u32> {
    let valid = (1..=9).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit());
    if valid { value.parse().ok() } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage() {
        for value in ["", "-1", "+1", "1.5", "abc", " 3", "1234567890"] {
            assert_eq!(parse_cart_count(value), None, "{value:?}");
        }
    }

    #[test]
    fn accepts_leading_zeros() {
        assert_eq!(parse_cart_count("007"), Some(7));
    }
}
