use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewBuffer, ViewBufferScope, ViewFirst, ViewSwap};

pin_project! {
    pub struct ScopeView<V> {
        #[pin]
        view: V,
        buffer: Option<Box<ViewBuffer>>,
        polled: bool,
    }
}

impl<V> ScopeView<V> {
    #[must_use]
    pub fn new(view: V) -> Self {
        Self {
            view,
            buffer: None,
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

        if !*this.polled && !ViewBufferScope::is_active() {
            *this.buffer = Some(Box::new(ViewBuffer::new()));
        }
        *this.polled = true;

        let poll = if this.buffer.is_some() {
            let _scope = ViewBufferScope::new(this.buffer);
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
        self.project().view.poll_swap(cx)
    }
}
