use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::Stream;
use http::{HeaderMap, StatusCode};
use http_body::Frame;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{BoxView, Swaps, View, ViewExt, ViewHandle, internal::MoveView};

use crate::{
    Body, BoxError,
    content::Html,
    response::{AsyncIntoResponse, IntoResponse, Response},
};

/// Replies with the rendered content as an HTML page, carrying the status
/// code and headers declared in it.
///
/// A handle holds a view's first content only, so nothing streams after the
/// page.
impl IntoResponse for ViewHandle {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let rendered = self.render_response(cx);
        html_response(cx, rendered.html, rendered.status_code, rendered.headers)
    }
}

/// Replies with the view's first content as an HTML page, then streams the
/// updates its live regions emit down the still-open body.
impl AsyncIntoResponse for BoxView<'static> {
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self, cx)
    }
}

/// Replies with the view's first content as an HTML page, then streams the
/// updates its live regions emit down the still-open body.
impl<Fut> AsyncIntoResponse for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self, cx)
    }
}

/// Resolves the view's first content into the response and hands the swaps
/// that follow to the body.
///
/// An error in the first content surfaces here, before any headers are sent,
/// so it becomes the error response like any handler error.
async fn stream<V: View + 'static>(view: V, cx: &Cx) -> Result<Response> {
    let (content, swaps) = view.live(cx).await?;
    let rendered = content.render_response(cx);
    let body = ViewBody {
        cx: cx.clone(),
        first: Some(rendered.html),
        swaps,
    };
    html_response(cx, Body::new(body), rendered.status_code, rendered.headers)
}

/// Builds an HTML response around `body`, applying the status code and
/// headers a view declared.
fn html_response(
    cx: &Cx,
    body: impl Into<Body>,
    status_code: Option<StatusCode>,
    headers: HeaderMap,
) -> Result<Response> {
    let mut response = Html(body.into()).into_response(cx)?;
    if let Some(status_code) = status_code {
        *response.status_mut() = status_code;
    }
    response.headers_mut().extend(headers);
    Ok(response)
}

/// The body of a view response: the rendered first content, then one frame
/// per swap.
struct ViewBody<V> {
    cx: Cx,
    /// The first content's HTML, emitted as the body's first frame.
    first: Option<String>,
    swaps: Swaps<V>,
}

impl<V: View + 'static> http_body::Body for ViewBody<V> {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        task: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if let Some(first) = this.first.take() {
            return Poll::Ready(Some(Ok(Frame::data(first.into()))));
        }
        match Pin::new(&mut this.swaps).poll_next(task) {
            // A swap streams down as an inert template plus a script
            // applying it to the swap's region.
            Poll::Ready(Some(Ok(swap))) => {
                let region = swap.region;
                let html = swap.replacement.render(&this.cx);
                let envelope = format!(
                    "<template data-tc-swap=\"{region}\">{html}</template>\
                     <script>topcoat.swap({region})</script>",
                );
                Poll::Ready(Some(Ok(Frame::data(envelope.into()))))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
    use topcoat::view::view;
    use topcoat_core::context::CxTestBuilder;

    use super::*;
    use crate::to_bytes;

    #[tokio::test]
    async fn view_handle_responds_with_its_html() {
        let cx = CxTestBuilder::new().build();
        let handle = view! { cx => <p>"hello"</p> }.first(&cx).await.unwrap();

        let response = handle.into_response(&cx).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "<p>hello</p>");
    }

    #[tokio::test]
    async fn view_handle_applies_declared_status_and_headers() {
        let cx = CxTestBuilder::new().build();
        let handle = view! { cx =>
            (StatusCode::NOT_FOUND)
            ((HeaderName::from_static("x-custom"), HeaderValue::from_static("yes")))
            <p>"missing"</p>
        }
        .first(&cx)
        .await
        .unwrap();

        let response = handle.into_response(&cx).unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers().get("x-custom").unwrap(), "yes");
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "<p>missing</p>");
    }

    #[tokio::test]
    async fn boxed_view_streams_its_first_content() {
        let cx = CxTestBuilder::new().build();
        let view = view! { cx =>
            (StatusCode::CREATED)
            <p>"streamed"</p>
        }
        .boxed();

        let response = view.async_into_response(&cx).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "<p>streamed</p>");
    }
}
