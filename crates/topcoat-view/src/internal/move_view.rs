use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::drive::{Emission, collect};
use crate::{Swap, View, buffer::ViewHandle};

pin_project! {
    /// A view owning the data of the scope it was built in.
    ///
    /// The `view!` macro wraps a template's body in this type. The body is
    /// an `async move` block: it captures every value the template uses, so
    /// the template has no lifetime tied to the scope it was written in.
    /// Inside the block, the body builds the template's view borrowing those
    /// captures and awaits [`drive`](super::drive), which polls it in place
    /// and tunnels its content and swaps out as this view's own. The built
    /// view never leaves the block, so its borrows stay valid for as long
    /// as the body lives.
    pub struct MoveView<Fut> {
        #[pin]
        body: Fut,
        done: bool,
    }
}

impl<Fut> MoveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self { body, done: false }
    }
}

impl<Fut> View for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let (poll, emission) = collect(this.body, cx);
        match emission {
            Some(Emission::Content(content)) => {
                if poll.is_ready() {
                    *this.done = true;
                }
                Poll::Ready(Ok(content))
            }
            Some(Emission::Swap(_)) => panic!("a `MoveView` body swapped before its first content"),
            None => match poll {
                Poll::Pending => Poll::Pending,
                // The body completed without driving a view; it renders
                // nothing and can never update.
                Poll::Ready(Ok(())) => {
                    *this.done = true;
                    Poll::Ready(Ok(ViewHandle::empty()))
                }
                Poll::Ready(Err(error)) => {
                    *this.done = true;
                    Poll::Ready(Err(error))
                }
            },
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        if *this.done {
            return Poll::Ready(None);
        }
        let (poll, emission) = collect(this.body, cx);
        match emission {
            Some(Emission::Swap(swap)) => {
                if poll.is_ready() {
                    *this.done = true;
                }
                Poll::Ready(Some(Ok(swap)))
            }
            Some(Emission::Content(_)) => {
                panic!("`poll_swap` called before `poll_first` returned `Ready`")
            }
            None => match poll {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(())) => {
                    *this.done = true;
                    Poll::Ready(None)
                }
                Poll::Ready(Err(error)) => {
                    *this.done = true;
                    Poll::Ready(Some(Err(error)))
                }
            },
        }
    }
}
