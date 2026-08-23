use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use crate::ViewChunk;
use topcoat_core::error::Result;

thread_local! {
    static YIELD: Cell<Option<Result<ViewChunk>>> = const { Cell::new(None) };
}

pub struct Yield {
    value: Option<Result<ViewChunk>>,
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.value.is_none() {
            return Poll::Ready(());
        }

        let current = YIELD.take();
        if current.is_some() {
            YIELD.set(current);
            return Poll::Pending;
        }

        YIELD.set(self.value.take());
        Poll::Pending
    }
}

#[must_use]
pub fn yield_(value: Result<ViewChunk>) -> Yield {
    Yield { value: Some(value) }
}

struct Collect {
    previous: Option<Result<ViewChunk>>,
}

impl Drop for Collect {
    fn drop(&mut self) {
        YIELD.set(self.previous.take());
    }
}

pub fn collect<F>(
    f: Pin<&mut F>,
    cx: &mut Context<'_>,
) -> (Poll<F::Output>, Option<Result<ViewChunk>>)
where
    F: Future,
{
    let _guard = Collect {
        previous: YIELD.take(),
    };
    let poll = f.poll(cx);
    let emit = YIELD.take();
    (poll, emit)
}
