use futures_util::StreamExt;
use pin_project_lite::pin_project;
use std::{
    pin::{Pin, pin},
    task::{Context, Poll},
};

use futures_core::{FusedStream, Stream};
use topcoat_core::error::Result;

use crate::{
    View, ViewChunk,
    yielder::{collect, yield_},
};

pub type BoxViewStream = Box<dyn Stream<Item = Result<ViewChunk>>>;

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
    #[doc(hidden)]
    pub fn new(f: F) -> Self {
        Self { f, done: false }
    }

    #[doc(hidden)]
    pub async fn yield_all(self) -> Result<()> {
        let mut this = pin!(self);
        while let Some(value) = this.next().await {
            yield_(Ok(value?)).await;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! emit {
    ($($tt:tt)*) => {
        $crate::ViewStream::yield_all($crate::view! { $($tt)* }).await
    };
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
