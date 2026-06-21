use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{
    Body, FromRequest, IntoResponse, OptionalFromRequest, Response, bad_request, content_type,
    to_bytes,
};

/// HTML request extractor and response wrapper.
///
/// As a response, wrap any value convertible into a [`Body`] (such as a
/// `String`) to reply with `Content-Type: text/html`. Rendered pages are
/// wrapped in `Html` automatically; use it directly from a
/// [`route`](macro@crate::route) that returns markup by hand.
///
/// As a request extractor, `Html<String>` requires a `Content-Type: text/html`
/// header and yields the body as text.
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Html<T>(pub T);

impl<T> From<T> for Html<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl FromRequest for Html<String> {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        if !html_content_type(content_type(cx)) {
            return Err(bad_request("expected request with `Content-Type: text/html`").into());
        }

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        let text = String::from_utf8(bytes.into())
            .map_err(|error| bad_request(format!("request body is not valid UTF-8: {error}")))?;

        Ok(Self(text))
    }
}

impl OptionalFromRequest for Html<String> {
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        if content_type(cx).is_some() {
            Ok(Some(<Self as FromRequest>::from_request(cx, body).await?))
        } else {
            Ok(None)
        }
    }
}

impl<T> IntoResponse for Html<T>
where
    T: Into<Body>,
{
    fn into_response(self) -> Result<Response> {
        (
            [(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            )],
            self.0.into(),
        )
            .into_response()
    }
}

fn html_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };

    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("text/html")
}
