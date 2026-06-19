use crate::runtime::Body;
use topcoat_core::runtime::context::Cx;
use topcoat_core::runtime::error::Result;

pub type Response<T = Body> = http::Response<T>;

/// Applies request-scoped side effects to a finished response before it is sent.
///
/// Currently this writes any pending cookie changes (with the `cookie` feature)
/// onto the response as `Set-Cookie` headers. It is a no-op otherwise.
#[inline]
pub(crate) fn finalize(cx: &Cx, response: Response) -> Response {
    #[cfg(feature = "cookie")]
    {
        let mut response = response;
        topcoat_cookie::write_cookies(cx, response.headers_mut());
        response
    }
    #[cfg(not(feature = "cookie"))]
    {
        let _ = cx;
        response
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

impl<T> IntoResponse for T
where
    T: axum::response::IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        Ok(axum::response::IntoResponse::into_response(self))
    }
}
