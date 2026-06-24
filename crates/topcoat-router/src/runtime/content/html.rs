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
/// [`route`](../topcoat_router_macro/attr.route.html) that returns markup by hand.
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

#[cfg(test)]
mod tests {
    use http::{Request, header::CONTENT_TYPE};
    use topcoat_core::runtime::context::Cx;

    use super::*;
    use crate::runtime::BadRequestError;

    /// Builds a `Cx` carrying request `Parts` with the given `Content-Type`
    /// header, or no header at all when `content_type` is [`None`].
    fn cx_with_content_type(content_type: Option<&str>) -> Cx {
        let mut builder = Request::builder();
        if let Some(content_type) = content_type {
            builder = builder.header(CONTENT_TYPE, content_type);
        }

        let (parts, ()) = builder.body(()).expect("request should build").into_parts();

        let mut cx = Cx::empty();
        cx.insert(parts);
        cx
    }

    #[tokio::test]
    async fn from_request_reads_html_body_as_text() {
        let cx = cx_with_content_type(Some("text/html; charset=utf-8"));
        let Html(text) =
            <Html<String> as FromRequest>::from_request(&cx, Body::from("<h1>hi</h1>"))
                .await
                .expect("a valid HTML body");

        assert_eq!(text, "<h1>hi</h1>");
    }

    #[tokio::test]
    async fn from_request_without_html_content_type_is_bad_request() {
        for content_type in [None, Some("application/json")] {
            let cx = cx_with_content_type(content_type);
            let error = <Html<String> as FromRequest>::from_request(&cx, Body::from("<p>x</p>"))
                .await
                .expect_err("a non-HTML content type is rejected");

            assert!(error.downcast_ref::<BadRequestError>().is_some());
        }
    }

    #[tokio::test]
    async fn from_request_rejects_non_utf8_body() {
        let cx = cx_with_content_type(Some("text/html"));
        let error = <Html<String> as FromRequest>::from_request(&cx, Body::from(vec![0xff, 0xfe]))
            .await
            .expect_err("an invalid UTF-8 body is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn optional_from_request_without_content_type_is_none() {
        let cx = cx_with_content_type(None);
        let html = <Html<String> as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("an absent content type is not an error");

        assert!(html.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_with_content_type_is_some() {
        let cx = cx_with_content_type(Some("text/html"));
        let html = <Html<String> as OptionalFromRequest>::from_request(&cx, Body::from("<p>x</p>"))
            .await
            .expect("a valid HTML body is not an error");

        assert_eq!(html.expect("an HTML payload is present").0, "<p>x</p>");
    }

    #[tokio::test]
    async fn into_response_sets_html_content_type() {
        let response = Html("<h1>hi</h1>")
            .into_response()
            .expect("response builds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"text/html; charset=utf-8".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"<h1>hi</h1>");
    }

    #[test]
    fn html_content_type_recognizes_html_media_types() {
        assert!(html_content_type(Some("text/html")));
        assert!(html_content_type(Some("text/html; charset=utf-8")));
        assert!(html_content_type(Some("TEXT/HTML")));

        assert!(!html_content_type(None));
        assert!(!html_content_type(Some("application/json")));
        assert!(!html_content_type(Some("text/plain")));
    }
}
