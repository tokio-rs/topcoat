//! Request-scoped, composable cookie jars.
//!
//! [`cookies`] returns the request's root [`CookieJar`]. [`Cookies`] is a trait
//! in the style of [`Iterator`]: its combinators wrap the jar in adapters that
//! sign, encrypt, prefix, or set default attributes, and each adapter is itself
//! a [`Cookies`] so they compose.
//!
//! An idiomatic pattern is to define your own `cookies(cx)` wrapper that bakes
//! in your app's policy:
//!
//! ```rust,ignore
//! use topcoat::{
//!     context::Cx,
//!     cookie::{Cookies, SameSite, signed_cookies},
//! };
//!
//! fn cookies(cx: &Cx) -> impl Cookies + '_ {
//!     signed_cookies(cx)
//!         .default_same_site(SameSite::Lax)
//!         .default_http_only(true)
//! }
//! ```
//!
//! Signing and encryption require a [`Key`] registered as app state with
//! `Router::app_state(Key::generate())`.

pub use topcoat_cookie::*;
