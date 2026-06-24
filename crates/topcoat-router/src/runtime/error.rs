mod bad_request;
mod forbidden;
mod internal_server;
mod method_not_allowed;
mod not_found;
mod redirect;
mod unauthorized;

pub use bad_request::*;
pub use forbidden::*;
pub use internal_server::*;
pub use method_not_allowed::*;
pub use not_found::*;
pub use redirect::*;
pub use unauthorized::*;

use http::StatusCode;

use crate::runtime::{Body, IntoResponse, Response};
use topcoat_core::runtime::error::{Error, Result};

/// Renders any [`IntoResponse`] value into a [`Response`], falling back to the
/// error's response if conversion fails. This is the terminal conversion the
/// router applies to a handler's return value.
pub(crate) fn respond(value: impl IntoResponse) -> Response {
    value.into_response().unwrap_or_else(error_into_response)
}

/// Maps the framework's error types onto their HTTP status codes, falling back
/// to a 500 for anything else.
fn error_into_response(error: Error) -> Response {
    macro_rules! try_downcast {
        ($ident:ident as $ty:ty) => {
            match $ident.downcast::<$ty>() {
                Ok(error) => return into_response_or_500(error),
                Err(error) => error,
            }
        };
    }
    let error = try_downcast!(error as ForbiddenError);
    let error = try_downcast!(error as BadRequestError);
    let error = try_downcast!(error as InternalServerError);
    let error = try_downcast!(error as NotFoundError);
    let error = try_downcast!(error as MethodNotAllowedError);
    let error = try_downcast!(error as RedirectError);
    let error = try_downcast!(error as UnauthorizedError);

    into_response_or_500(internal_server_error(error))
}

/// Renders an error response, falling back to a bare 500 (none of the error
/// types' responses can actually fail to build).
fn into_response_or_500(value: impl IntoResponse) -> Response {
    value.into_response().unwrap_or_else(|_| {
        let mut response = Response::new(Body::from("internal server error"));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    })
}

/// Renders the contained value, or the framework error response on `Err`.
impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        match self {
            Ok(value) => value.into_response(),
            Err(error) => Ok(error_into_response(error)),
        }
    }
}

/// Renders an error by mapping it onto its HTTP status code.
impl IntoResponse for Error {
    fn into_response(self) -> Result<Response> {
        Ok(error_into_response(self))
    }
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
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::Result;
/// use topcoat::context::Cx;
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
    ///
    /// # Errors
    ///
    /// Returns a [`RedirectError`] performing a temporary redirect to `uri`
    /// when the value is absent.
    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError>;

    /// Returns `Ok(value)` if present, otherwise a permanent redirect to `uri`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedirectError`] performing a permanent redirect to `uri`
    /// when the value is absent.
    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError>;

    /// Returns `Ok(value)` if present, otherwise a not-found response.
    ///
    /// # Errors
    ///
    /// Returns a [`NotFoundError`] when the value is absent.
    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError>;

    /// Returns `Ok(value)` if present, otherwise an unauthorized response.
    ///
    /// # Errors
    ///
    /// Returns an [`UnauthorizedError`] when the value is absent.
    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError>;

    /// Returns `Ok(value)` if present, otherwise a forbidden response.
    ///
    /// # Errors
    ///
    /// Returns a [`ForbiddenError`] when the value is absent.
    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError>;

    /// Returns `Ok(value)` if present, otherwise a bad-request response.
    ///
    /// # Errors
    ///
    /// Returns a [`BadRequestError`] carrying `description` when the value is
    /// absent.
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
