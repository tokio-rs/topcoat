use std::{
    cell::Cell,
    future::Ready,
    mem,
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
            let _guard = YieldGuard::new();
            (this.body.poll(cx), YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Yield::First(first)) => Poll::Ready(Ok(first)),
            (Poll::Pending, Yield::NotSet) => Poll::Pending,
            (Poll::Pending, Yield::Swap(_)) => {
                panic!("move view future yielded a swap before its first content")
            }
            (Poll::Ready(_), Yield::First(_) | Yield::Swap(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), Yield::NotSet) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), Yield::NotSet) => {
                panic!("move view future completed without yielding anything")
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let (poll, yielded) = {
            let _guard = YieldGuard::new();
            (this.body.poll(cx), YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Yield::Swap(swap)) => Poll::Ready(Ok(Some(swap))),
            (Poll::Pending, Yield::NotSet) => Poll::Pending,
            (Poll::Pending, Yield::First(_)) => {
                panic!("move view future yielded first content twice")
            }
            (Poll::Ready(_), Yield::First(_) | Yield::Swap(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), Yield::NotSet) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), Yield::NotSet) => Poll::Ready(Ok(None)),
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
                    YIELD.set(Yield::First(first));
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(Some(swap))) => {
                    YIELD.set(Yield::Swap(swap));
                    Poll::Pending
                }
                Poll::Ready(Ok(None)) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

thread_local! {
    /// What the view driven on this task handed back on its last poll.
    ///
    /// A driven view reports out of band because it is polled through a
    /// future, which has no room for a value in its pending state. The slot
    /// is read back by the poll that set it going, so it only ever holds a
    /// value across a single poll.
    static YIELD: Cell<Yield> = const { Cell::new(Yield::NotSet) };
}

/// The value a driven view handed back, if any.
#[derive(Default)]
enum Yield {
    /// The view has not reported since the slot was last read.
    #[default]
    NotSet,
    /// The view's first content.
    First(ViewFirst),
    /// An update to content the view already reported.
    Swap(ViewSwap),
}

/// Keeps the yield slot of an enclosing poll while a nested one runs.
struct YieldGuard {
    prev: Yield,
}

impl YieldGuard {
    fn new() -> Self {
        Self { prev: YIELD.take() }
    }
}

impl Drop for YieldGuard {
    fn drop(&mut self) {
        YIELD.set(mem::take(&mut self.prev));
    }
}
