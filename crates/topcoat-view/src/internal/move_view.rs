use std::{
    future::Ready,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::yielder::{DriveFuture, Yield, poll_body};
use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    /// A [`View`] polled through an async body that owns data the view
    /// borrows.
    ///
    /// A top-level `view!` and a captured control-flow body expand to one:
    /// the body moves the values the template captures into itself, builds
    /// the nested view, and drives it in place, so the view's borrows stay
    /// alive for as long as it runs. What the driven view resolves passes
    /// through out of band, one value per poll.
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
    /// Drives `view` inside a move body, handing its first content and
    /// every swap after it to the enclosing poll; resolves once the view
    /// has no further updates.
    pub fn drive<V: View>(view: V) -> impl Future<Output = Result<()>> {
        DriveFuture::new(view)
    }
}

impl<Fut> View for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();

        match poll_body(this.body, cx) {
            (Poll::Pending, Some(Yield::First(first))) => Poll::Ready(Ok(first)),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Pending, Some(Yield::Swap(_))) => {
                panic!("move view future yielded a swap before its first content")
            }
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

        match poll_body(this.body, cx) {
            (Poll::Pending, Some(Yield::Swap(swap))) => Poll::Ready(Ok(Some(swap))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Pending, Some(Yield::First(_))) => {
                panic!("move view future yielded first content twice")
            }
            (Poll::Ready(_), Some(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), None) => Poll::Ready(Ok(None)),
        }
    }
}
