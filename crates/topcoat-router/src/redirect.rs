//! Redirects modeled as errors.
//!
//! A [`RedirectError`] can be returned as the `Err` variant of a handler's
//! [`Result`], short-circuiting the request and sending a redirect response.
//! [`redirect`] and [`redirect_permanent`] construct one directly, and
//! [`RedirectExt`] lets `Option` and `Result` values fall through to a
//! redirect via the `?` operator.

use axum::response::Redirect;

use crate::Result;

/// Builds a temporary (HTTP 307) redirect to `uri`.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::router::{Result, redirect};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(redirect("/users").into());
///     };
///     Ok(user)
/// }
/// ```
pub fn redirect(uri: &str) -> RedirectError {
    RedirectError::new(Redirect::temporary(uri))
}

/// Builds a permanent (HTTP 308) redirect to `uri`.
///
/// Use this for URLs that have moved for good — clients and search engines
/// are allowed to cache the new location.
pub fn redirect_permanent(uri: &str) -> RedirectError {
    RedirectError::new(Redirect::permanent(uri))
}

/// A redirect response carried as the `Err` variant of a handler [`Result`].
///
/// Construct one with [`redirect`] or [`redirect_permanent`], or surface one
/// from an `Option` / `Result` via [`RedirectExt`].
#[derive(Debug)]
pub struct RedirectError {
    inner: axum::response::Redirect,
}

impl RedirectError {
    fn new(inner: axum::response::Redirect) -> Self {
        Self { inner }
    }
}

impl axum::response::IntoResponse for RedirectError {
    fn into_response(self) -> axum::response::Response {
        self.inner.into_response()
    }
}

/// Converts an absent or failed value into a redirect.
///
/// Implemented for [`Option`] (where `None` becomes a redirect) and
/// [`Result`] (where any `Err` becomes a redirect, discarding the original
/// error). Designed to be combined with `?` so a caller can fall through
/// to a redirect on missing or invalid state.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::router::{Result, RedirectExt};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let user = lookup(cx, id).await.ok_or_redirect("/users")?;
///     Ok(user)
/// }
/// ```
pub trait RedirectExt {
    /// The success type produced when the value is present.
    type T;

    /// Returns `Ok(value)` if present, otherwise a temporary redirect to `uri`.
    fn ok_or_redirect(self, uri: &str) -> Result<Self::T>;

    /// Returns `Ok(value)` if present, otherwise a permanent redirect to `uri`.
    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T>;
}

impl<T> RedirectExt for Option<T> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T> {
        match self {
            Some(value) => Ok(value),
            None => Err(redirect(uri).into()),
        }
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T> {
        match self {
            Some(value) => Ok(value),
            None => Err(redirect_permanent(uri).into()),
        }
    }
}

impl<T, E> RedirectExt for Result<T, E> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(redirect(uri).into()),
        }
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(redirect_permanent(uri).into()),
        }
    }
}
