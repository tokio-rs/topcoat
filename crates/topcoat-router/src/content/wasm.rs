use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// WebAssembly response wrapper.
///
/// Wrap any value convertible into a [`Body`] (such as a `&'static [u8]` from
/// `include_bytes!`) to reply with `Content-Type: application/wasm`. Use it
/// from a [`route`](../topcoat_router_macro/attr.route.html) that serves a
/// module by hand.
///
/// The media type is load-bearing here. `WebAssembly.compileStreaming` and
/// `instantiateStreaming` reject any response that does not carry exactly
/// `application/wasm`, so a module served as `application/octet-stream` fails
/// to instantiate even though the bytes are correct.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{content::Wasm, route},
/// };
/// # const ENGINE: &[u8] = b"\0asm\x01\0\0\0";
///
/// #[route(GET "/engine.wasm")]
/// async fn engine() -> Result<Wasm<&'static [u8]>> {
///     Ok(Wasm(ENGINE))
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Wasm<T>(pub T);

impl<T> From<T> for Wasm<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> IntoResponse for Wasm<T>
where
    T: Into<Body>,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/wasm"))],
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
    async fn into_response_sets_wasm_content_type() {
        // The four-byte module preamble: `\0asm` and version 1.
        let response = Wasm(b"\0asm\x01\0\0\0".as_slice())
            .into_response(&Cx::default())
            .expect("response builds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"application/wasm".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"\0asm\x01\0\0\0");
    }

    /// The streaming entry points match the media type exactly, so a charset
    /// parameter would break them the way `application/octet-stream` does.
    #[tokio::test]
    async fn into_response_sends_the_media_type_without_parameters() {
        let response = Wasm(b"\0asm\x01\0\0\0".as_slice())
            .into_response(&Cx::default())
            .expect("response builds");

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("a content type is set");

        assert!(!content_type.contains(';'), "{content_type}");
    }
}
