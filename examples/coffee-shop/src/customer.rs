//! The returning customer, read from a cookie.
//!
//! Topcoat prefers plain `cx: &Cx` functions over middlewares and extractors:
//! any handler, layout, component, or procedure that wants the customer calls
//! [`current_customer`], and nothing is threaded through the router for it.

use topcoat::{
    context::Cx,
    cookie::{Cookies, cookie, cookies, time::Duration},
};

const COOKIE: &str = "customer";

/// The name of the returning customer, if the browser sent one.
pub fn current_customer(cx: &Cx) -> Option<String> {
    let cookie = cookies(cx).get(COOKIE)?;
    let name = cookie.value().trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// Remembers the customer's name for a month.
pub fn remember_customer(cx: &Cx, name: &str) {
    cookies(cx).add(cookie! {
        COOKIE = name.to_owned();
        Path = "/";
        HttpOnly;
        SameSite = Lax;
        MaxAge = Duration::days(30)
    });
}

/// Forgets the customer's name.
pub fn forget_customer(cx: &Cx) {
    cookies(cx).remove(cookie!(COOKIE = ""; Path = "/"));
}
