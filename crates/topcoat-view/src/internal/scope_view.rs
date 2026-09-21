use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewBuffer, ViewBufferScope, ViewFirst, ViewSwap};

pin_project! {
    /// Installs a buffer for a [`View`]'s content while it is polled.
    ///
    /// A scope first polled with no build running owns its buffer and seals
    /// its first content with it, making the content self-contained. Polled
    /// inside a running build it defers to that build's buffer, and its
    /// content splices into the enclosing view.
    pub struct ScopeView<V> {
        #[pin]
        view: V,
        buffer: Option<Box<ViewBuffer>>,
        polled: bool,
    }
}

impl<V> ScopeView<V> {
    /// Creates a scope that decides on first poll whether it needs a
    /// buffer of its own.
    #[must_use]
    pub fn new(view: V) -> Self {
        Self {
            view,
            buffer: None,
            polled: false,
        }
    }

    /// Creates a scope that owns its buffer from the start.
    ///
    /// The buffer is installed while `f` builds the view, so everything the
    /// build pushes lands in the same buffer the polls install again.
    #[must_use]
    pub fn self_contained(f: impl FnOnce() -> V) -> Self {
        let mut buffer = Some(Box::new(ViewBuffer::new()));
        let view = {
            let _buffer = ViewBufferScope::new(&mut buffer);
            f()
        };
        Self {
            view,
            buffer,
            polled: false,
        }
    }
}

impl<V> View for ScopeView<V>
where
    V: View,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();

        if !*this.polled && !this.buffer.is_some() && !ViewBufferScope::is_active() {
            *this.buffer = Some(Box::new(ViewBuffer::new()));
        }
        *this.polled = true;

        let poll = if this.buffer.is_some() {
            let _buffer = ViewBufferScope::new(this.buffer);
            this.view.poll_first(cx)
        } else {
            this.view.poll_first(cx)
        };

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(ViewFirst {
                content,
                streaming,
                connecting,
            })) => {
                let content = if let Some(buffer) = this.buffer.take() {
                    content.seal(*buffer)
                } else {
                    content
                };
                Poll::Ready(Ok(ViewFirst {
                    content,
                    streaming,
                    connecting,
                }))
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        this.view.poll_swap(cx)
    }
}
