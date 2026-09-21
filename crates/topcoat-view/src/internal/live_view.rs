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
    EmitToken, RegionId, View, ViewBufferScope, ViewFirst, ViewPass, ViewSwap,
    internal::yielder::{poll_first, poll_swap},
};

pin_project! {
    pub struct LiveView<I, C> {
        #[pin]
        initial: I,
        #[pin]
        connected: C,
    }
}

impl<I, C> LiveView<I, C>
where
    I: Future<Output = Result<EmitToken>>,
    C: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    pub fn new(initial: I, connected: C) -> Self {
        Self { initial, connected }
    }
}

impl LiveView<Ready<Result<EmitToken>>, Ready<Result<EmitToken>>> {
    pub fn drive<V: View>(view: V) -> impl Future<Output = Result<EmitToken>> {
        DriveFuture::new(view).map_ok(|()| EmitToken)
    }
}

impl<I, C> View for LiveView<I, C>
where
    I: Future<Output = Result<EmitToken>> + Send,
    C: Future<Output = Result<EmitToken>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();

        match poll_first(this.initial.as_mut(), cx) {
            (Poll::Pending, Some(first)) => Poll::Ready(Ok(first)),
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

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pass: ViewPass,
    ) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        match pass {
            ViewPass::Initial => match poll_swap(this.initial, cx, pass) {
                (Poll::Pending, Some(swap)) => Poll::Ready(Ok(Some(swap))),
                (Poll::Pending, None) => Poll::Pending,
                (Poll::Ready(_), Some(_)) => {
                    panic!("live view future yielded without returning pending")
                }
                (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
                (Poll::Ready(Ok(_)), None) => Poll::Ready(Ok(None)),
            },
            ViewPass::Connected => match poll_swap(this.connected, cx, pass) {
                (Poll::Pending, Some(swap)) => Poll::Ready(Ok(Some(swap))),
                (Poll::Pending, None) => Poll::Pending,
                (Poll::Ready(_), Some(_)) => {
                    panic!("live view future yielded without returning pending")
                }
                (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
                (Poll::Ready(Ok(_)), None) => Poll::Ready(Ok(None)),
            },
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
                Poll::Ready(Ok(ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|parts| {
                            parts.push_region_start(*this.region);
                            parts.push_view_handle(first.content);
                            parts.push_region_end(*this.region);
                        })
                    }),
                    ..first
                }))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pass: ViewPass,
    ) -> Poll<Result<Option<ViewSwap>>> {
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
            this.view.poll_swap(cx, pass)
        }
    }
}
