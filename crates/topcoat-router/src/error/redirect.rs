use std::borrow::Cow;

use http::{HeaderValue, StatusCode, header::LOCATION};
use percent_encoding::{CONTROLS, utf8_percent_encode};
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::response::{IntoResponse, Response};

/// Builds a temporary (HTTP 307) redirect to `uri`.
///
/// Characters a URI cannot carry, like non-ASCII ones, are percent-encoded.
/// Percent signs already in `uri` are left alone, so an encoded target is not
/// encoded twice.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::{Result, context::Cx, router::error::redirect};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(redirect("/users").into());
///     };
///     Ok(user)
/// }
/// ```
#[must_use]
pub fn redirect(uri: impl AsRef<str>) -> RedirectError {
    RedirectError::new(StatusCode::TEMPORARY_REDIRECT, uri.as_ref())
}

/// Builds a permanent (HTTP 308) redirect to `uri`.
///
/// Use this for URLs that have moved for good; clients and search engines
/// are allowed to cache the new location. The target is percent-encoded like
/// [`redirect`] does.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{error::redirect_permanent, page},
/// };
///
/// // The page always redirects, so it renders no view of its own.
/// #[page]
/// async fn legacy_profile(cx: &Cx) -> Result<()> {
///     Err(redirect_permanent("/profile").into())
/// }
/// ```
#[must_use]
pub fn redirect_permanent(uri: impl AsRef<str>) -> RedirectError {
    RedirectError::new(StatusCode::PERMANENT_REDIRECT, uri.as_ref())
}

/// A redirect response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`redirect`] or [`redirect_permanent`], or derive one
/// from an `Option` / `Result` via [`RouterErrorExt`](crate::error::RouterErrorExt).
/// For the Post/Redirect/Get pattern, which sends the browser on with a
/// `GET`, reach for [`see_other`] instead.
#[derive(Debug)]
pub struct RedirectError {
    status: StatusCode,
    location: HeaderValue,
}

impl RedirectError {
    /// Builds a redirect with the given status code and target `uri`.
    fn new(status: StatusCode, uri: &str) -> Self {
        Self {
            status,
            location: location(uri),
        }
    }

    /// The target the redirect points at.
    #[cfg(test)]
    pub(crate) fn location(&self) -> &HeaderValue {
        &self.location
    }
}

impl std::fmt::Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("redirect")
    }
}

impl std::error::Error for RedirectError {}

impl IntoResponse for RedirectError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status, ([(LOCATION, self.location)], ())).into_response(cx)
    }
}

/// Builds a "see other" (HTTP 303) redirect to `uri`.
///
/// Unlike [`redirect`] and [`redirect_permanent`], which preserve the request
/// method, a 303 tells the client to follow `uri` with a `GET`. Reply with it
/// after a successful `POST`, `PUT`, or `DELETE` to land the browser on a page
/// -- the Post/Redirect/Get pattern that keeps a reload from re-submitting the
/// mutation. The target is percent-encoded like [`redirect`] does.
///
/// A route returns the redirect as its `Ok` value. A page renders a view, so
/// it returns the redirect through `Err` instead, like the other redirects.
///
/// # Examples
///
/// From a route:
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{
///         error::{SeeOther, see_other},
///         route,
///     },
/// };
///
/// #[route(POST "/logout")]
/// async fn logout(cx: &Cx) -> Result<SeeOther> {
///     // ...clear the session...
///     Ok(see_other("/"))
/// }
/// ```
///
/// From a page:
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{content::Form, error::see_other, page},
///     view::{View, view},
/// };
/// # use serde::Deserialize;
/// # #[derive(Deserialize)]
/// # struct Signup { email: String }
/// # async fn create_account(_cx: &Cx, _email: &str) -> Result<bool> { Ok(true) }
///
/// #[page(POST "/signup")]
/// async fn signup(cx: &Cx, Form(input): Form<Signup>) -> Result<impl View> {
///     if create_account(cx, &input.email).await? {
///         return Err(see_other("/welcome").into());
///     }
///     Ok(view! { <p>"That email is already taken."</p> })
/// }
/// ```
#[must_use]
pub fn see_other(uri: impl AsRef<str>) -> SeeOther {
    SeeOther::new(uri.as_ref())
}

/// A "see other" (HTTP 303) redirect response.
///
/// It is the Post/Redirect/Get reply for a completed `POST`, `PUT`, or
/// `DELETE`, sending the browser to a new location with a `GET`. Construct one
/// with [`see_other`].
///
/// The redirect is both a response and an error, so a handler can return it
/// either way: a route returns it through `Ok`, and a page, whose `Ok` value
/// is a view, returns it through `Err`. Both produce the same 303 response.
#[derive(Debug)]
pub struct SeeOther {
    location: HeaderValue,
}

impl SeeOther {
    /// Builds a 303 redirect to `uri`.
    fn new(uri: &str) -> Self {
        Self {
            location: location(uri),
        }
    }

    /// The target the redirect points at.
    #[cfg(test)]
    pub(crate) fn location(&self) -> &HeaderValue {
        &self.location
    }
}

impl std::fmt::Display for SeeOther {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("see other")
    }
}

impl std::error::Error for SeeOther {}

impl IntoResponse for SeeOther {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::SEE_OTHER, ([(LOCATION, self.location)], ())).into_response(cx)
    }
}

/// Extracts the target of a redirect carried as an error, handing any other
/// error back unchanged.
///
/// # Errors
///
/// Returns `Err(error)` if the error is not a redirect.
pub(crate) fn redirect_location(error: Error) -> Result<HeaderValue, Error> {
    let error = match error.downcast::<RedirectError>() {
        Ok(redirect) => return Ok(redirect.location),
        Err(error) => error,
    };
    error
        .downcast::<SeeOther>()
        .map(|redirect| redirect.location)
}

/// Builds the `Location` header value pointing at `uri`.
///
/// Control and non-ASCII characters are percent-encoded, which leaves only
/// the printable ASCII a header value and a URI both accept. That keeps the
/// value convertible back to a `str`, which a redirect sent mid-stream relies
/// on.
fn location(uri: &str) -> HeaderValue {
    let uri: Cow<'_, str> = utf8_percent_encode(uri, CONTROLS).into();
    HeaderValue::try_from(&*uri).expect("percent-encoded uri is a valid header value")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_targets_are_kept_as_is() {
        assert_eq!(
            redirect("/users?page=2#top").location(),
            "/users?page=2#top"
        );
    }

    #[test]
    fn non_ascii_targets_are_percent_encoded() {
        assert_eq!(redirect("/caf\u{e9}").location(), "/caf%C3%A9");
        assert_eq!(
            redirect_permanent("https://\u{4f8b}\u{3048}.jp/").location(),
            "https://%E4%BE%8B%E3%81%88.jp/"
        );
        assert_eq!(see_other("/caf\u{e9}").location(), "/caf%C3%A9");
    }

    #[test]
    fn control_characters_are_percent_encoded() {
        assert_eq!(redirect("/a\r\nb\x7f").location(), "/a%0D%0Ab%7F");
    }

    #[test]
    fn encoded_targets_are_not_encoded_twice() {
        assert_eq!(
            redirect("/caf%C3%A9?q=a%20b").location(),
            "/caf%C3%A9?q=a%20b"
        );
    }

    #[test]
    fn the_location_converts_back_to_a_str() {
        assert!(redirect("/caf\u{e9} \u{4f8b}").location().to_str().is_ok());
    }

    #[test]
    fn the_location_of_a_redirect_error_is_extracted() {
        assert_eq!(
            redirect_location(redirect("/target").into()).unwrap(),
            "/target"
        );
        assert_eq!(
            redirect_location(see_other("/target").into()).unwrap(),
            "/target"
        );
    }

    #[test]
    fn other_errors_are_handed_back() {
        let error = redirect_location(std::fmt::Error.into()).unwrap_err();
        assert!(error.downcast_ref::<std::fmt::Error>().is_some());
    }
}
