use pin_project_lite::pin_project;
use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use topcoat_core::error::Result;

use crate::View;

pub struct ViewChunk {
    id: u64,
    view: View,
}

pin_project! {
    pub struct ViewStream<F> {
        #[pin]
        f: F,
    }
}

impl<F> Stream for ViewStream<F>
where
    F: Future<Output = ()>,
{
    type Item = Result<ViewChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.f.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => Poll::Ready(None),
        }
    }
}
