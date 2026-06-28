use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{
    Body, FromRequest, IntoResponse, OptionalFromRequest, Response, bad_request, content_type,
    to_bytes,
};

/// CSS request extractor and response wrapper.
///
/// As a response, wrap any value convertible into a [`Body`] (such as a
/// `String`) to reply with `Content-Type: text/css`. Use it directly from a
/// [`route`](../topcoat_router_macro/attr.route.html) that returns a stylesheet
/// by hand.
///
/// As a request extractor, `Css<String>` requires a `Content-Type: text/css`
/// header and yields the body as text.
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Css<T>(pub T);

impl<T> From<T> for Css<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl FromRequest for Css<String> {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        if !css_content_type(content_type(cx)) {
            return Err(bad_request("expected request with `Content-Type: text/css`").into());
        }

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        let text = String::from_utf8(bytes.into())
            .map_err(|error| bad_request(format!("request body is not valid UTF-8: {error}")))?;

        Ok(Self(text))
    }
}

impl OptionalFromRequest for Css<String> {
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        if content_type(cx).is_some() {
            Ok(Some(<Self as FromRequest>::from_request(cx, body).await?))
        } else {
            Ok(None)
        }
    }
}

impl<T> IntoResponse for Css<T>
where
    T: Into<Body>,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (
            [(
                CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            )],
            self.0.into(),
        )
            .into_response(cx)
    }
}

fn css_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };

    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("text/css")
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
    async fn from_request_reads_css_body_as_text() {
        let cx = cx_with_content_type(Some("text/css; charset=utf-8"));
        let Css(text) =
            <Css<String> as FromRequest>::from_request(&cx, Body::from("body { color: red; }"))
                .await
                .expect("a valid CSS body");

        assert_eq!(text, "body { color: red; }");
    }

    #[tokio::test]
    async fn from_request_without_css_content_type_is_bad_request() {
        for content_type in [None, Some("application/json")] {
            let cx = cx_with_content_type(content_type);
            let error = <Css<String> as FromRequest>::from_request(&cx, Body::from("body {}"))
                .await
                .expect_err("a non-CSS content type is rejected");

            assert!(error.downcast_ref::<BadRequestError>().is_some());
        }
    }

    #[tokio::test]
    async fn from_request_rejects_non_utf8_body() {
        let cx = cx_with_content_type(Some("text/css"));
        let error = <Css<String> as FromRequest>::from_request(&cx, Body::from(vec![0xff, 0xfe]))
            .await
            .expect_err("an invalid UTF-8 body is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn optional_from_request_without_content_type_is_none() {
        let cx = cx_with_content_type(None);
        let css = <Css<String> as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("an absent content type is not an error");

        assert!(css.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_with_content_type_is_some() {
        let cx = cx_with_content_type(Some("text/css"));
        let css = <Css<String> as OptionalFromRequest>::from_request(&cx, Body::from("body {}"))
            .await
            .expect("a valid CSS body is not an error");

        assert_eq!(css.expect("a CSS payload is present").0, "body {}");
    }

    #[tokio::test]
    async fn into_response_sets_css_content_type() {
        let response = Css("body { color: red; }")
            .into_response(&Cx::empty())
            .expect("response builds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"text/css; charset=utf-8".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"body { color: red; }");
    }

    #[test]
    fn css_content_type_recognizes_css_media_types() {
        assert!(css_content_type(Some("text/css")));
        assert!(css_content_type(Some("text/css; charset=utf-8")));
        assert!(css_content_type(Some("TEXT/CSS")));

        assert!(!css_content_type(None));
        assert!(!css_content_type(Some("application/json")));
        assert!(!css_content_type(Some("text/plain")));
    }
}