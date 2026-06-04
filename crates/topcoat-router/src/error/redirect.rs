use crate::Response;

/// Builds a temporary (HTTP 307) redirect to `uri`.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::redirect;
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(redirect("/users").into());
///     };
///     Ok(user)
/// }
/// ```
pub fn redirect(uri: &str) -> RedirectError {
    RedirectError::new(axum::response::Redirect::temporary(uri))
}

/// Builds a permanent (HTTP 308) redirect to `uri`.
///
/// Use this for URLs that have moved for good — clients and search engines
/// are allowed to cache the new location.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::{page, redirect_permanent};
///
/// #[page]
/// async fn legacy_profile(cx: &Cx) -> Result {
///     Err(redirect_permanent("/profile").into())
/// }
/// ```
pub fn redirect_permanent(uri: &str) -> RedirectError {
    RedirectError::new(axum::response::Redirect::permanent(uri))
}

/// A redirect response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`redirect`] or [`redirect_permanent`], or derive one
/// from an `Option` / `Result` via [`crate::RouterErrorExt`].
#[derive(Debug)]
pub struct RedirectError {
    inner: axum::response::Redirect,
}

impl RedirectError {
    fn new(inner: axum::response::Redirect) -> Self {
        Self { inner }
    }
}

impl std::fmt::Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("redirect")
    }
}

impl std::error::Error for RedirectError {}

impl axum::response::IntoResponse for RedirectError {
    fn into_response(self) -> Response {
        self.inner.into_response()
    }
}
