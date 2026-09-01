use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{RegionScope, View, ViewBuffer, ViewBufferScope, ViewFirst, ViewSwap};

pin_project! {
    pub struct ScopeView<V> {
        #[pin]
        view: V,
        buffer: Option<Box<ViewBuffer>>,
        regions: u64,
        polled: bool,
    }
}

impl<V> ScopeView<V> {
    #[must_use]
    pub fn new(view: V) -> Self {
        Self {
            view,
            buffer: None,
            regions: 1,
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
            regions: 1,
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

        let _regions = RegionScope::new(this.regions);
        let poll = if this.buffer.is_some() {
            let _buffer = ViewBufferScope::new(this.buffer);
            this.view.poll_first(cx)
        } else {
            this.view.poll_first(cx)
        };

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(ViewFirst { content, live })) => {
                let content = if let Some(buffer) = this.buffer.take() {
                    content.seal(*buffer)
                } else {
                    content
                };
                Poll::Ready(Ok(ViewFirst { content, live }))
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let _regions = RegionScope::new(this.regions);
        this.view.poll_swap(cx)
    }
}
