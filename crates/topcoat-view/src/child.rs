use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{BoxView, Step, View};

/// The child content a component invocation passes to its component.
///
/// The children of an invocation reach the component as this view in its
/// props. The component decides where they render by interpolating the
/// value into its own template, which drives the children concurrently
/// with the rest of the template; children that are never interpolated
/// never run.
pub struct Child<'a> {
    view: BoxView<'a>,
}

impl<'a> Child<'a> {
    /// Wraps a view as a component's child content.
    #[must_use]
    pub fn new(view: impl View + 'a) -> Self {
        Self {
            view: Box::pin(view),
        }
    }
}

/// No child content: renders nothing, like a component invoked without
/// children.
impl Default for Child<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

/// The children's view polls through in place.
impl View for Child<'_> {
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        self.get_mut().view.as_mut().poll(cx)
    }
}
