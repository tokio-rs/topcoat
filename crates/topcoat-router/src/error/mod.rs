mod bad_request;
mod forbidden;
mod internal_server;
mod not_found;
mod redirect;
mod unauthorized;

pub use bad_request::*;
pub use forbidden::*;
pub use internal_server::*;
pub use not_found::*;
pub use redirect::*;
pub use unauthorized::*;

use crate::{IntoResponse, Response};
use topcoat_core::error::{Error, Result};

/// Turns a Result into a response.
///
/// The IntoResponse trait unfortunately cannot be implemented on [`Result`] because it would clash
/// with the axum implementation.
pub(crate) fn result_into_response<T: IntoResponse>(result: Result<T>) -> Response {
    match result {
        Ok(value) => value.into_response().unwrap_or_else(error_into_response),
        Err(error) => error_into_response(error),
    }
}

/// Turns an Error into a response.
///
/// The IntoResponse trait unfortunately cannot be implemented on [`Error`] because it would clash
/// with the axum implementation.
pub(crate) fn error_into_response(error: Error) -> Response {
    macro_rules! try_downcast {
        ($ident:ident as $ty:ty) => {
            match $ident.downcast::<$ty>() {
                Ok(error) => return axum::response::IntoResponse::into_response(error),
                Err(error) => error,
            }
        };
    }
    let error = try_downcast!(error as ForbiddenError);
    let error = try_downcast!(error as BadRequestError);
    let error = try_downcast!(error as InternalServerError);
    let error = try_downcast!(error as NotFoundError);
    let error = try_downcast!(error as RedirectError);
    let error = try_downcast!(error as UnauthorizedError);

    axum::response::IntoResponse::into_response(internal_server_error(error))
}

/// Converts an absent or failed value into a router error response.
///
/// Implemented for [`Option`] (where `None` becomes the configured error)
/// and [`core::result::Result`] (where any `Err` is replaced, discarding the
/// original error). Designed to be combined with `?` so a handler can return a
/// redirect, not-found, unauthorized, forbidden, or bad-request response when
/// required state is missing or invalid.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::RouterErrorExt;
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let user = lookup(cx, id).await.ok_or_redirect("/users")?;
///     Ok(user)
/// }
/// ```
pub trait RouterErrorExt {
    /// The success type produced when the value is present.
    type T;

    /// Returns `Ok(value)` if present, otherwise a temporary redirect to `uri`.
    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError>;

    /// Returns `Ok(value)` if present, otherwise a permanent redirect to `uri`.
    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError>;

    /// Returns `Ok(value)` if present, otherwise a not-found response.
    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError>;

    /// Returns `Ok(value)` if present, otherwise an unauthorized response.
    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError>;

    /// Returns `Ok(value)` if present, otherwise a forbidden response.
    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError>;

    /// Returns `Ok(value)` if present, otherwise a bad-request response.
    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError>;
}

impl<T> RouterErrorExt for Option<T> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError> {
        match self {
            Some(value) => Ok(value),
            None => Err(redirect(uri)),
        }
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError> {
        match self {
            Some(value) => Ok(value),
            None => Err(redirect_permanent(uri)),
        }
    }

    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError> {
        match self {
            Some(value) => Ok(value),
            None => Err(not_found()),
        }
    }

    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError> {
        match self {
            Some(value) => Ok(value),
            None => Err(unauthorized()),
        }
    }

    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError> {
        match self {
            Some(value) => Ok(value),
            None => Err(forbidden()),
        }
    }

    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError> {
        match self {
            Some(value) => Ok(value),
            None => Err(bad_request(description)),
        }
    }
}

impl<T, E> RouterErrorExt for Result<T, E> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(redirect(uri)),
        }
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(redirect_permanent(uri)),
        }
    }

    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(not_found()),
        }
    }

    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(unauthorized()),
        }
    }

    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(forbidden()),
        }
    }

    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(bad_request(description)),
        }
    }
}
