use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::{FusedStream, Stream};
use topcoat_core::error::{Error, Result};

use crate::{ViewChunk, yielder::collect};

/// A lazy view: a stream of rendered [`ViewChunk`]s.
///
/// A `view!` invocation evaluates to a value implementing this trait, and a
/// component returns one as `Result<impl View>`. The view does no work until
/// it is polled; rendering it means driving the stream and writing each
/// chunk out.
///
/// A view is either the unboxed value `view!` produces or a [`BoxView`]
/// erasing it behind an allocation, obtained with [`boxed`](View::boxed).
pub trait View: Stream<Item = Result<ViewChunk>> + Send {
    /// Erases the view's concrete type behind a boxed one.
    ///
    /// Every `view!` invocation has its own anonymous type, so a function
    /// returning `impl View` from multiple `return` sites must box each view
    /// to give them a common type.
    fn boxed<'cx>(self) -> BoxView<'cx>
    where
        Self: Sized + 'cx,
    {
        Box::pin(self)
    }
}

/// A [`View`] erased behind a boxed, pinned trait object.
pub type BoxView<'cx> = Pin<Box<dyn View + 'cx>>;

impl View for BoxView<'_> {}

pin_project! {
    /// The [`View`] a `view!` invocation produces: a stream of
    /// [`ViewChunk`]s driven by the future the template compiled to.
    pub struct ViewStream<F> {
        #[pin]
        f: F,
        done: bool,
        error: Option<Error>,
    }
}

impl<F> ViewStream<F>
where
    F: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(f: F) -> Self {
        Self {
            f,
            done: false,
            error: None,
        }
    }
}

impl<F> View for ViewStream<F> where F: Future<Output = Result<()>> + Send {}

impl<F> Stream for ViewStream<F>
where
    F: Future<Output = Result<()>>,
{
    type Item = Result<ViewChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        let this = self.project();
        // An error the future completed with while a chunk was still in
        // flight; it becomes the stream's final item.
        if let Some(error) = this.error.take() {
            *this.done = true;
            return Poll::Ready(Some(Err(error)));
        }
        let (poll, value) = collect(this.f, cx);
        match poll {
            Poll::Pending => match value {
                Some(value) => Poll::Ready(Some(value)),
                None => Poll::Pending,
            },
            Poll::Ready(Ok(())) => {
                *this.done = true;
                Poll::Ready(value)
            }
            Poll::Ready(Err(error)) => {
                if let Some(value) = value {
                    *this.error = Some(error);
                    Poll::Ready(Some(value))
                } else {
                    *this.done = true;
                    Poll::Ready(Some(Err(error)))
                }
            }
        }
    }
}

impl<F> FusedStream for ViewStream<F>
where
    F: Future<Output = Result<()>>,
{
    fn is_terminated(&self) -> bool {
        self.done
    }
}
