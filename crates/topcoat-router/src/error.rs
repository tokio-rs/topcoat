#![doc = include_str!("../docs/error.md")]

mod bad_request;
mod content_too_large;
mod forbidden;
mod internal_server;
mod method_not_allowed;
mod not_found;
mod redirect;
mod rewrite;
mod service_unavailable;
mod too_many_requests;
mod unauthorized;

pub use bad_request::*;
pub use content_too_large::*;
pub use forbidden::*;
use http::StatusCode;
pub use internal_server::*;
pub use method_not_allowed::*;
pub use not_found::*;
pub use redirect::*;
pub use rewrite::*;
pub use service_unavailable::*;
pub use too_many_requests::*;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};
pub use unauthorized::*;

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// Renders any [`IntoResponse`] value into a [`Response`], falling back to the
/// error's response if conversion fails. This is the terminal conversion the
/// router applies to a handler's return value.
pub(crate) fn respond(cx: &Cx, value: impl IntoResponse) -> Response {
    value
        .into_response(cx)
        .unwrap_or_else(|error| error_into_response(cx, error))
}

/// Builds a bare 500 response without consulting request or application code.
pub(crate) fn internal_server_response() -> Response {
    let mut response = Response::new(Body::from("internal server error"));
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response
}

/// Maps the framework's error types onto their HTTP status codes, falling back
/// to a 500 for anything else.
fn error_into_response(cx: &Cx, error: Error) -> Response {
    macro_rules! try_downcast {
        ($ident:ident as $ty:ty) => {
            match $ident.downcast_cloned::<$ty>() {
                Ok(error) => return into_response_or_500(cx, error),
                Err(error) => error,
            }
        };
    }
    let error = try_downcast!(error as ForbiddenError);
    let error = try_downcast!(error as BadRequestError);
    let error = try_downcast!(error as ContentTooLargeError);
    let error = try_downcast!(error as InternalServerError);
    let error = try_downcast!(error as NotFoundError);
    let error = try_downcast!(error as MethodNotAllowedError);
    let error = try_downcast!(error as RedirectError);
    let error = try_downcast!(error as SeeOther);
    let error = try_downcast!(error as UnauthorizedError);
    let error = try_downcast!(error as ServiceUnavailableError);
    let error = try_downcast!(error as TooManyRequestsError);

    into_response_or_500(cx, internal_server_error(error))
}

/// Renders an error response, falling back to a bare 500 (none of the error
/// types' responses can actually fail to build).
fn into_response_or_500(cx: &Cx, value: impl IntoResponse) -> Response {
    value
        .into_response(cx)
        .unwrap_or_else(|_| internal_server_response())
}

/// Renders the `Ok` value, or the error response for the `Err` value.
impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        match self {
            Ok(value) => value.into_response(cx),
            Err(error) => Ok(error_into_response(cx, error)),
        }
    }
}

/// Renders an error as the response of its router error type, such as a
/// `404 Not Found` for a [`NotFoundError`]. Any other error renders as a
/// `500 Internal Server Error`.
impl IntoResponse for Error {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        Ok(error_into_response(cx, self))
    }
}

/// Turns a missing or failed value into a router error.
///
/// For an [`Option`], `None` becomes the chosen error. For a
/// [`Result`](core::result::Result), any `Err` becomes the chosen error and the
/// original error is discarded. Use these methods with `?` to return a
/// redirect or an error status when a value a handler needs is missing.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::{Result, context::Cx, router::error::RouterErrorExt};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let user = lookup(cx, id).await.ok_or_redirect("/users")?;
///     Ok(user)
/// }
/// ```
pub trait RouterErrorExt {
    /// The type of the value when it is present.
    type T;

    /// Returns the value if present, otherwise a temporary redirect to `uri`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedirectError`] with status `307 Temporary Redirect` when
    /// the value is missing.
    fn ok_or_redirect(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError>;

    /// Returns the value if present, otherwise a permanent redirect to `uri`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedirectError`] with status `308 Permanent Redirect` when
    /// the value is missing.
    fn ok_or_redirect_permanent(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError>;

    /// Returns the value if present, otherwise a `404 Not Found` error.
    ///
    /// # Errors
    ///
    /// Returns a [`NotFoundError`] when the value is missing.
    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError>;

    /// Returns the value if present, otherwise a `401 Unauthorized` error.
    ///
    /// # Errors
    ///
    /// Returns an [`UnauthorizedError`] when the value is missing.
    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError>;

    /// Returns the value if present, otherwise a `403 Forbidden` error.
    ///
    /// # Errors
    ///
    /// Returns a [`ForbiddenError`] when the value is missing.
    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError>;

    /// Returns the value if present, otherwise a `400 Bad Request` error.
    ///
    /// # Errors
    ///
    /// Returns a [`BadRequestError`] with `description` when the value is
    /// missing.
    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError>;
}

impl<T> RouterErrorExt for Option<T> {
    type T = T;

    fn ok_or_redirect(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError> {
        match self {
            Some(value) => Ok(value),
            None => Err(redirect(uri)),
        }
    }

    fn ok_or_redirect_permanent(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError> {
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

    fn ok_or_redirect(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(redirect(uri)),
        }
    }

    fn ok_or_redirect_permanent(self, uri: impl AsRef<str>) -> Result<Self::T, RedirectError> {
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

#[cfg(test)]
mod tests {
    use http::header::RETRY_AFTER;

    use super::*;

    /// The mapping is a closed list of downcasts, so an error type that is not
    /// on it degrades to a 500 no matter what its own `IntoResponse` says. A
    /// shed answered as "broken" rather than "busy" is the failure this guards.
    #[test]
    fn a_service_unavailable_error_maps_to_503_not_500() {
        let error: Error = service_unavailable(2).into();

        let response = error_into_response(&Cx::default(), error);

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response
                .headers()
                .get(RETRY_AFTER)
                .map(http::HeaderValue::as_bytes),
            Some(&b"2"[..])
        );
    }

    /// The 429 mirror: a rate limit answered as "the server is broken" is the
    /// failure this guards.
    #[test]
    fn a_too_many_requests_error_maps_to_429_not_500() {
        let error: Error = too_many_requests(60).into();

        let response = error_into_response(&Cx::default(), error);

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get(RETRY_AFTER)
                .map(http::HeaderValue::as_bytes),
            Some(&b"60"[..])
        );
    }

    #[test]
    fn an_error_that_is_not_on_the_list_still_maps_to_500() {
        let error: Error = std::io::Error::other("boom").into();

        let response = error_into_response(&Cx::default(), error);

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// A memoized loader keeps its error in the request cache while the
    /// handler returns a clone, so the mapping must recover the status from
    /// a shared error too. A cached "not found" answered as "broken" is the
    /// failure this guards.
    #[test]
    fn a_shared_error_still_maps_to_its_status() {
        let error: Error = not_found().into();
        let cached = error.clone();

        let response = error_into_response(&Cx::default(), error);

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        drop(cached);
    }
}
