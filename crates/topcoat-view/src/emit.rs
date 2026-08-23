use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use crate::stream::ViewChunk;
use topcoat_core::error::Result;

thread_local! {
    static EMIT: Cell<Option<Result<ViewChunk>>> = const { Cell::new(None) };
}

pub struct Emit {
    value: Option<Result<ViewChunk>>,
}

impl Emit {
    fn new(value: Option<Result<ViewChunk>>) -> Self {
        Self { value }
    }
}

impl Future for Emit {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.value.is_none() {
            return Poll::Ready(());
        }

        let current = EMIT.take();
        if current.is_some() {
            EMIT.set(current);
            return Poll::Pending;
        }

        EMIT.set(self.value.take());
        Poll::Pending
    }
}

#[must_use]
pub fn emit(value: Result<ViewChunk>) -> Emit {
    Emit { value: Some(value) }
}

struct Collect {
    previous: Option<Result<ViewChunk>>,
}

impl Drop for Collect {
    fn drop(&mut self) {
        EMIT.set(self.previous.take());
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
        previous: EMIT.take(),
    };
    let poll = f.poll(cx);
    let emit = EMIT.take();
    (poll, emit)
}
