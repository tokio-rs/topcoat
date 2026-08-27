use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use topcoat_core::error::Result;

use super::Pending;
use crate::{Step, View, ViewHandle};

/// A template whose block was built, as a [`View`]: the views at its
/// reserved node positions are driven into their slots, and their updates
/// then merge into one stream of swaps.
///
/// The block is reported as the view's content once every position was
/// filled, so the handle a caller receives always renders.
pub struct TemplateView<P> {
    /// The block; taken once it is reported as the content.
    content: Option<ViewHandle>,
    pending: P,
}

impl<P: Pending> TemplateView<P> {
    #[must_use]
    pub fn new(content: ViewHandle, pending: P) -> Self {
        Self {
            content: Some(content),
            pending,
        }
    }
}

impl<P> View for TemplateView<P>
where
    P: Pending + Send,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.get_mut();
        if this.content.is_some() {
            ready!(this.pending.poll_fill(cx))?;
            let content = this.content.take().expect("the content is still to report");
            return Poll::Ready(Ok(Step::Content {
                content,
                live: this.pending.is_live(),
            }));
        }
        this.pending.poll_swap(cx)
    }
}
