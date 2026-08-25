use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::{FusedStream, Stream};
use topcoat_core::error::{Error, Result};

use crate::{
    buffer::{ViewBuffer, ViewBufferScope, ViewHandle},
    yielder::collect,
};

pub struct RegionId(usize);

pub struct Swap {
    region: RegionId,
    replacement: ViewHandle,
}

pub trait View: Send {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>>;

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>>;

    /// Erases the view's concrete type behind a boxed one.
    ///
    /// Every `view!` invocation has its own anonymous type, so a function
    /// returning `impl View` from multiple `return` sites must box each view
    /// to give them a common type.
    fn boxed<'a>(self) -> BoxView<'a>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }
}

/// A [`View`] erased behind a boxed, pinned trait object.
pub type BoxView<'a> = Pin<Box<dyn View + 'a>>;

impl View for BoxView<'_> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        self.as_deref_mut().poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        self.as_deref_mut().poll_swap(cx)
    }
}

pin_project! {
    /// The [`View`] a `view!` invocation produces: a stream of
    /// [`ViewChunk`]s driven by the future the template compiled to.
    pub struct ViewStream<F> {
        #[pin]
        f: F,
        done: bool,
        error: Option<Error>,
        role: Role,
        // A root stream's buffer between polls; installed on the task for
        // the duration of each poll, and moved into the content chunk when
        // it is yielded.
        buffer: Option<Box<ViewBuffer>>,
    }
}

/// Who owns the buffer a stream's views are built in, decided at the first
/// poll.
///
/// A stream polled with a buffer already installed is nested: it appends to
/// the enclosing stream's buffer, and its chunks stay cheap handles into it.
/// Otherwise the stream is the root: it installs a buffer of its own for
/// exactly the duration of each poll, and seals it into the content chunk it
/// yields.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Undecided,
    Root,
    Nested,
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
            role: Role::Undecided,
            buffer: None,
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
        if *this.role == Role::Undecided {
            *this.role = if ViewBufferScope::is_active() {
                Role::Nested
            } else {
                *this.buffer = Some(Box::new(ViewBuffer::new()));
                Role::Root
            };
        }
        let (poll, value) = if *this.role == Role::Root {
            let _scope = ViewBufferScope::swap(this.buffer);
            collect(this.f, cx)
        } else {
            collect(this.f, cx)
        };
        // A root stream's content chunk takes the buffer it was built in
        // with it, so it is self-contained once it leaves the stream.
        let value = match value {
            Some(Ok(ViewChunk::Content(view))) if *this.role == Role::Root => Some(Ok(
                ViewChunk::Content(view.seal(this.buffer.take().map(|buffer| *buffer))),
            )),
            other => other,
        };
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
