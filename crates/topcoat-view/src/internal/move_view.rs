use std::{
    cell::Cell,
    future::Ready,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    pub struct MoveView<Fut> {
        #[pin]
        body: Fut,
    }
}

impl<Fut> MoveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self { body }
    }
}

impl MoveView<Ready<()>> {
    pub fn drive<V: View>(view: V) -> impl Future<Output = Result<()>> {
        DriveFuture { view, first: true }
    }
}

impl<Fut> View for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        let (poll, yielded) = {
            let _guard = FirstGuard::new();
            (this.body.poll(cx), FIRST_YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Some(value)) => Poll::Ready(Ok(value)),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), None) => {
                panic!("move view future completed without yielding anything")
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let (poll, yielded) = {
            let _guard = SwapGuard::new();
            (this.body.poll(cx), SWAP_YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Some(value)) => Poll::Ready(Ok(Some(value))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), None) => Poll::Ready(Ok(None)),
        }
    }
}

pin_project! {
    struct DriveFuture<V> {
        #[pin]
        view: V,
        first: bool,
    }
}

impl<V> Future for DriveFuture<V>
where
    V: View,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.first {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(first)) => {
                    *this.first = false;
                    FIRST_YIELD.set(Some(first));
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(swap)) => {
                    SWAP_YIELD.set(swap);
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

thread_local! {
    static FIRST_YIELD: Cell<Option<ViewFirst>> = const { Cell::new(None) };
}

struct FirstGuard {
    prev: Option<ViewFirst>,
}

impl FirstGuard {
    fn new() -> Self {
        Self {
            prev: FIRST_YIELD.take(),
        }
    }
}

impl Drop for FirstGuard {
    fn drop(&mut self) {
        FIRST_YIELD.replace(self.prev.take());
    }
}

thread_local! {
    static SWAP_YIELD: Cell<Option<ViewSwap>> = const { Cell::new(None) };
}

struct SwapGuard {
    prev: Option<ViewSwap>,
}

impl SwapGuard {
    fn new() -> Self {
        Self {
            prev: SWAP_YIELD.take(),
        }
    }
}

impl Drop for SwapGuard {
    fn drop(&mut self) {
        SWAP_YIELD.replace(self.prev.take());
    }
}
