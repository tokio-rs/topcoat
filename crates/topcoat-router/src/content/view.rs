use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::future::poll_fn;
use http::{HeaderMap, StatusCode};
use http_body::Frame;
use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{BoxView, View, ViewExt, ViewHandle, internal::MoveView};

use crate::{
    Body, BoxError,
    content::Html,
    response::{AsyncIntoResponse, IntoResponse, Response},
};

impl IntoResponse for ViewHandle {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let rendered = self.render_response(cx);
        html_response(cx, rendered.html, rendered.status_code, rendered.headers)
    }
}

impl AsyncIntoResponse for BoxView<'static> {
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self, cx)
    }
}

impl<Fut> AsyncIntoResponse for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self.boxed(), cx)
    }
}

async fn stream<V: View + Unpin + 'static>(mut view: V, cx: &Cx) -> Result<Response> {
    let mut pinned_view = Pin::new(&mut view);
    let first = poll_fn(|cx| pinned_view.as_mut().poll_first(cx)).await?;
    let rendered = first.content.render_response(cx);
    if first.live {
        let body = ViewBody {
            cx: cx.clone(),
            first: Some(rendered.html),
            script_sent: false,
            done: false,
            view,
        };
        html_response(cx, Body::new(body), rendered.status_code, rendered.headers)
    } else {
        html_response(
            cx,
            Body::new(rendered.html),
            rendered.status_code,
            rendered.headers,
        )
    }
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

/// Applies streamed swaps in the browser: replaces the content between a
/// region's marker comments with the template the swap arrived in.
///
/// Sent once per streaming response, ahead of the first swap. The `??=`
/// guard steps aside for an applier the page installed itself.
const SWAP_SCRIPT: &str = r"<script>
window.topcoat ??= {
    swap(id) {
        const script = document.currentScript;
        const template = script.previousElementSibling;
        let open = null;
        let close = null;
        const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_COMMENT);
        while (walker.nextNode()) {
            const comment = walker.currentNode;
            if (comment.data === `topcoat::region::start(${id})`) open = comment;
            else if (comment.data === `topcoat::region::end(${id})`) close = comment;
        }
        if (open && close) {
            while (open.nextSibling && open.nextSibling !== close) open.nextSibling.remove();
            close.parentNode.insertBefore(template.content, close);
        }
        template.remove();
        script.remove();
    },
};
</script>";

pin_project! {
    struct ViewBody<V> {
        cx: Cx,
        first: Option<String>,
        // Whether the swap applier script was already sent, which happens
        // ahead of the first swap.
        script_sent: bool,
        // Whether the view has reported it has no further swaps. Polling a
        // view past that point resumes a future that already completed.
        done: bool,
        #[pin]
        view: V,
    }
}

impl<V: View + 'static> http_body::Body for ViewBody<V> {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        if let Some(first) = this.first.take() {
            return Poll::Ready(Some(Ok(Frame::data(first.into()))));
        }
        if *this.done {
            return Poll::Ready(None);
        }
        match this.view.poll_swap(cx) {
            Poll::Ready(Ok(Some(swap))) => {
                let script = if *this.script_sent { "" } else { SWAP_SCRIPT };
                *this.script_sent = true;
                let region = swap.region;
                let html = swap.replacement.render(this.cx);
                let envelope = format!(
                    "{script}<template data-topcoat-swap=\"{region}\">{html}</template>\
                     <script>topcoat.swap({region})</script>",
                );
                Poll::Ready(Some(Ok(Frame::data(envelope.into()))))
            }
            Poll::Ready(Ok(None)) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                Poll::Ready(Some(Err(error.into())))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.first.is_none() && self.done
    }
}
