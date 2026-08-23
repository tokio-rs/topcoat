use futures_util::StreamExt;
use pin_project_lite::pin_project;
use std::{
    pin::{Pin, pin},
    task::{Context, Poll},
};

use futures_core::{FusedStream, Stream};
use topcoat_core::error::Result;

use crate::{
    View,
    yielder::{collect, yield_},
};

pub struct ViewChunk {
    id: u64,
    view: View,
}

pin_project! {
    pub struct ViewStream<F> {
        #[pin]
        f: F,
        done: bool,
    }
}

impl<F> ViewStream<F>
where
    F: Future<Output = ()>,
{
    pub async fn yield_all(self) {
        let mut this = pin!(self);
        while let Some(value) = this.next().await {
            yield_(value).await;
        }
    }
}

impl<F> Stream for ViewStream<F>
where
    F: Future<Output = ()>,
{
    type Item = Result<ViewChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        let this = self.project();
        let (poll, value) = collect(this.f, cx);
        if let Some(value) = value {
            return Poll::Ready(Some(value));
        }
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => {
                *this.done = true;
                Poll::Ready(None)
            }
        }
    }
}

impl<F> FusedStream for ViewStream<F>
where
    F: Future<Output = ()>,
{
    fn is_terminated(&self) -> bool {
        self.done
    }
}
