use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::Stream;
use http_body::Frame;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{BoxView, Swaps, View, ViewExt, internal::MoveView};

use crate::{
    Body, BoxError,
    content::Html,
    response::{AsyncIntoResponse, IntoResponse, Response},
};

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
    let mut response = Html(Body::new(body)).into_response(cx)?;
    if let Some(status_code) = rendered.status_code {
        *response.status_mut() = status_code;
    }
    response.headers_mut().extend(rendered.headers);
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
