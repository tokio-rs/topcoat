use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::drive::{Emission, collect};
use crate::{Step, View, buffer::ViewHandle};

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
        // Whether the first content was yielded.
        started: bool,
    }
}

impl<Fut> MoveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self {
            body,
            started: false,
        }
    }
}

impl<Fut> View for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        let (poll, emission) = collect(this.body, cx);
        let live = poll.is_pending();
        match emission {
            Some(Emission::Content(content)) => {
                assert!(!*this.started, "a `MoveView` body drove content twice");
                *this.started = true;
                Poll::Ready(Ok(Step::Content { content, live }))
            }
            Some(Emission::Swap(swap)) => {
                assert!(
                    *this.started,
                    "a `MoveView` body swapped before its first content"
                );
                Poll::Ready(Ok(Step::Swap { swap, live }))
            }
            None => match poll {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(())) if *this.started => Poll::Ready(Ok(Step::Done)),
                // The body completed without driving a view; it renders
                // nothing and can never update.
                Poll::Ready(Ok(())) => Poll::Ready(Ok(Step::Content {
                    content: ViewHandle::empty(),
                    live: false,
                })),
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            },
        }
    }
}
