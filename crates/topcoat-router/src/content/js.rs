use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// JavaScript response wrapper.
///
/// `Js<T>` wraps any value convertible into a [`Body`], such as a `String`,
/// and replies with `Content-Type: text/javascript; charset=utf-8`. Use it
/// from a `#[route]` that serves a script by hand instead of as a static
/// asset.
///
/// Browsers only run a `<script type="module">` when its response carries a
/// JavaScript media type, so module scripts need this header.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{content::Js, route},
/// };
///
/// #[route(GET "/app.js")]
/// async fn app_js() -> Result<Js<&'static str>> {
///     Ok(Js("export const ready = true;"))
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Js<T>(pub T);

impl<T> From<T> for Js<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> IntoResponse for Js<T>
where
    T: Into<Body>,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (
            [(
                CONTENT_TYPE,
                HeaderValue::from_static("text/javascript; charset=utf-8"),
            )],
            self.0.into(),
        )
            .into_response(cx)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;
    use crate::to_bytes;

    #[tokio::test]
    async fn into_response_sets_javascript_content_type() {
        let response = Js("export const a = 1;")
            .into_response(&Cx::default())
            .expect("response builds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"text/javascript; charset=utf-8".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"export const a = 1;");
    }
}
