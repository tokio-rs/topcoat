use std::{
    fmt,
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
};

use topcoat_core::error::Result;

use crate::{
    View,
    buffer::{CellId, ViewBuffer, ViewBufferScope},
    live::Fill,
};

/// One live render: the root future, the render scope it builds in, and the
/// root cell its fill delivers the document to.
///
/// The driver owns the scope between polls and installs it around each one.
/// Each poll sweeps the render until a pass makes no progress, so internal
/// completions propagate within the poll while external futures suspend the
/// task through their own wakers.
pub struct LiveRender<F> {
    future: Pin<Box<F>>,
    /// The render scope between polls; installed for the duration of each.
    buffer: Option<Box<ViewBuffer>>,
    /// The root cell the render's fill delivers the document to.
    root: CellId,
}

impl<F> LiveRender<F>
where
    F: Future<Output = Result<()>> + Send,
{
    /// Creates the render: a fresh scope, a root cell, and the root future
    /// minted from the fill that delivers to it.
    pub fn new(render: impl FnOnce(Fill) -> F) -> Self {
        let mut buffer = Box::new(ViewBuffer::new());
        let root = buffer.new_cell(true);
        Self {
            future: Box::pin(render(Fill::new(root))),
            buffer: Some(buffer),
            root,
        }
    }

    /// Drives the render until nothing inside it can change anymore and
    /// returns the final document, the mode for contexts that do not stream.
    ///
    /// A render with nothing deferred completes on its first pass; one that
    /// defers completes when the last deferred region resolved, and the
    /// returned view is the document a streaming client would have ended up
    /// with.
    ///
    /// # Errors
    ///
    /// Returns the first error nothing in the render caught: the error a
    /// page or layout let bubble to the root.
    ///
    /// # Panics
    ///
    /// Panics if the render future completes while a poll still holds the
    /// scope, which the driver never does.
    pub async fn to_completion(mut self) -> Result<View> {
        poll_fn(|task| {
            loop {
                let buffer = self
                    .buffer
                    .as_deref_mut()
                    .expect("the driver owns the buffer between polls");
                buffer.clear_progress();
                let poll =
                    ViewBufferScope::install(&mut self.buffer, || self.future.as_mut().poll(task));
                match poll {
                    Poll::Ready(result) => return Poll::Ready(result),
                    Poll::Pending => {
                        let progressed = self
                            .buffer
                            .as_deref()
                            .expect("the driver owns the buffer between polls")
                            .progress();
                        if !progressed {
                            return Poll::Pending;
                        }
                    }
                }
            }
        })
        .await?;
        let buffer = *self
            .buffer
            .take()
            .expect("the driver owns the buffer between polls");
        let view = buffer
            .delivered_view(self.root)
            .ok_or(RenderNeverDelivered)?;
        Ok(view.seal(Some(buffer)))
    }
}

/// The error reported when a render future completed without delivering its
/// document through the root fill.
#[derive(Debug)]
struct RenderNeverDelivered;

impl fmt::Display for RenderNeverDelivered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the render completed without delivering a document")
    }
}

impl std::error::Error for RenderNeverDelivered {}
