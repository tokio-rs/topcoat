use std::{
    future::Ready,
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
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: RegionId,
        pass: ViewPass,
        connected: bool,
        connecting: bool,
        started: bool,
        done: bool,
        stash: Option<ViewSwap>,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    pub fn new(
        identity: Identity,
        site: SiteKey,
        pass: ViewPass,
        body: impl FnOnce(RegionId) -> Fut,
    ) -> Self {
        let mut view = Self::branches(identity, site, pass, true, body);
        view.connecting = false;
        view
    }

    #[doc(hidden)]
    pub fn branches(
        identity: Identity,
        site: SiteKey,
        pass: ViewPass,
        connected: bool,
        body: impl FnOnce(RegionId) -> Fut,
    ) -> Self {
        let region = RegionId::new(identity, site);
        Self {
            body: body(region),
            region,
            pass,
            connected,
            connecting: connected && pass == ViewPass::Initial,
            started: false,
            done: false,
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
        *this.started = true;

        match poll_first(this.body.as_mut(), cx) {
            (Poll::Pending, Some(first)) => {
                // The emitted child can be settled while its body still has
                // more emissions. Poll the body again to determine liveness.
                let (poll, yielded) = poll_swap(this.body, cx);
                if let Poll::Ready(Err(error)) = poll {
                    return Poll::Ready(Err(error));
                }
                *this.done = poll.is_ready();
                *this.stash = yielded;
                let streaming = !*this.done;
                let connecting = *this.connecting || first.connecting;
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

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        if *this.done {
            return Poll::Ready(Ok(None));
        }
        if !*this.started && *this.pass == ViewPass::Connected && !*this.connected {
            *this.done = true;
            return Poll::Ready(Ok(None));
        }
        *this.started = true;
        if let Some(swap) = this.stash.take() {
            return Poll::Ready(Ok(Some(swap)));
        }

        match poll_swap(this.body, cx) {
            (Poll::Pending, Some(swap)) => Poll::Ready(Ok(Some(swap))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(result), None) => {
                *this.done = true;
                Poll::Ready(result.map(|_| None))
            }
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

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
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
            this.view.poll_swap(cx)
        }
    }
}
