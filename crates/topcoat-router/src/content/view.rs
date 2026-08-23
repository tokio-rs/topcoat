use std::{pin::Pin, task::Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use http_body::Frame;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{View, ViewChunk};

use crate::{
    Body, BoxError,
    content::Html,
    response::{IntoResponse, Response},
};

pub struct ViewResponse {
    first: View,
    rest: Pin<Box<dyn Stream<Item = Result<ViewChunk>> + Send>>,
}

impl ViewResponse {
    pub async fn try_from(
        mut stream: Pin<Box<dyn Stream<Item = Result<ViewChunk>> + Send>>,
    ) -> Result<Self> {
        Ok(Self {
            first: stream
                .next()
                .await
                .ok_or_else(|| panic!("view did not emit anything"))??
                .view,
            rest: stream,
        })
    }
}

impl IntoResponse for ViewResponse {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let rendered = self.first.render_response(cx);
        let mut response = Html(Body::new(ViewBody {
            cx: cx.clone(),
            first: rendered.html,
            rest: self.rest,
        }))
        .into_response(cx)?;
        if let Some(status_code) = rendered.status_code {
            *response.status_mut() = status_code;
        }
        response.headers_mut().extend(rendered.headers);
        Ok(response)
    }
}

struct ViewBody {
    cx: Cx,
    first: String,
    rest: Pin<Box<dyn Stream<Item = Result<ViewChunk>> + Send>>,
}

impl http_body::Body for ViewBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.rest.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                Poll::Ready(Some(Ok(Frame::data(chunk.view.render(&self.cx).into()))))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
