use std::{
    future::Ready,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::TryFutureExt;
use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::yielder::DriveFuture;
use crate::{
    EmitToken, RegionId, View, ViewBufferScope, ViewFirst, ViewSwap,
    internal::yielder::{poll_first, poll_swap},
};

pin_project! {
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: RegionId,
        has_connected: bool,
        first_polled: bool,
        stash: Option<ViewSwap>,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    pub fn new(region: RegionId, has_connected: bool, body: Fut) -> Self {
        Self {
            body,
            region,
            has_connected,
            first_polled: false,
            stash: None,
        }
    }
}

impl LiveView<Ready<Result<EmitToken>>> {
    pub fn drive<V: View>(region: RegionId, view: V) -> impl Future<Output = Result<EmitToken>> {
        DriveFuture::new(EmitView::new(region, view)).map_ok(|()| EmitToken)
    }
}

impl<Fut> View for LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();
        *this.first_polled = true;
        match poll_first(this.body.as_mut(), cx) {
            (Poll::Pending, Some(first)) => {
                // The emitted child can be settled while its body still has
                // more emissions. Poll the body again to determine liveness.
                let (poll, yielded) = poll_swap(this.body, cx);
                if let Poll::Ready(Err(error)) = poll {
                    return Poll::Ready(Err(error));
                }
                *this.stash = yielded;
                let streaming = poll.is_pending();
                let connecting = *this.has_connected || first.connecting;
                let content = if streaming || connecting {
                    ViewBufferScope::with(|buffer| {
                        buffer.block(|parts| {
                            parts.push_region_start(*this.region);
                            parts.push_view_handle(first.content);
                            parts.push_region_end(*this.region);
                        })
                    })
                } else {
                    first.content
                };
                Poll::Ready(Ok(ViewFirst {
                    content,
                    streaming,
                    connecting,
                }))
            }
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(_)), None) => {
                panic!("live view future completed without yielding anything")
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        if !*this.first_polled && !*this.has_connected {
            return Poll::Ready(Ok(None));
        }

        if let Some(swap) = this.stash.take() {
            return Poll::Ready(Ok(Some(swap)));
        }

        match poll_swap(this.body, cx) {
            (Poll::Pending, Some(swap)) => Poll::Ready(Ok(Some(swap))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(result), None) => Poll::Ready(result.map(|_| None)),
        }
    }
}

pin_project! {
    pub struct EmitView<V> {
        #[pin]
        view: V,
        region: RegionId,
        first: bool,
    }
}

impl<V> EmitView<V> {
    pub fn new(region: RegionId, view: V) -> Self {
        Self {
            view,
            region,
            first: true,
        }
    }
}

impl<V> View for EmitView<V>
where
    V: View,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        match this.view.poll_first(cx) {
            Poll::Ready(Ok(first)) => {
                *this.first = false;
                Poll::Ready(Ok(first))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        if *this.first {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(first)) => {
                    *this.first = false;
                    Poll::Ready(Ok(Some(ViewSwap {
                        region: *this.region,
                        replacement: first.content,
                    })))
                }
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            this.view.poll_swap(cx)
        }
    }
}
