use std::{
    future::Ready,
    panic::Location,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::TryFutureExt;
use pin_project_lite::pin_project;
use topcoat_core::{
    error::Result,
    identity::{Identity, SiteKey},
};

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
        region: RegionId,
    }
}

impl<I, C> LiveView<I, C>
where
    I: Future<Output = Result<EmitToken>>,
    C: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    #[track_caller]
    pub fn new(identity: Identity, initial: I, connected: C) -> Self {
        Self {
            initial,
            connected,
            region: RegionId::new(identity, SiteKey::from_location(Location::caller())),
        }
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
            (Poll::Pending, Some(first)) => Poll::Ready(Ok(ViewFirst {
                content: ViewBufferScope::with(|buffer| {
                    buffer.block(|parts| {
                        parts.push_region_start(*this.region);
                        parts.push_view_handle(first.content);
                        parts.push_region_end(*this.region);
                    })
                }),
                streaming: first.streaming,
                connecting: first.connecting,
            })),
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
