use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// JavaScript response wrapper.
///
/// Wrap any value convertible into a [`Body`] (such as a `String`) to reply
/// with `Content-Type: text/javascript`. Use it from a
/// [`route`](../topcoat_router_macro/attr.route.html) that serves a script by
/// hand, rather than as a bundled [`asset`](crate).
///
/// The media type is not cosmetic for module scripts: a browser refuses to
/// execute `<script type="module">` whose response does not carry a JavaScript
/// media type, and reports it as a MIME type mismatch rather than a script
/// error.
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
