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

use super::yielder::{DriveFuture, Yield, poll_body};
use crate::{EmitToken, Pass, RegionId, View, ViewBufferScope, ViewFirst, ViewHandle, ViewSwap};

pin_project! {
    /// A `live!` region as a [`View`]: a body future whose emissions become
    /// the region's content.
    ///
    /// The body reports each emission out of band while it runs. The first
    /// one becomes the view's first content; when the body is already done
    /// at that point and nothing connects later, the content is final and
    /// needs no markers. Otherwise the content is framed with the markers of
    /// its stable region and every later emission becomes a swap of that
    /// region.
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: RegionId,
        stash: Option<ViewSwap>,
        // The pass the region runs in.
        pass: Pass,
        // Whether the region has a connected phase to run after the page
        // has loaded.
        connecting: bool,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>>,
{
    /// A region with one body serving both phases.
    #[doc(hidden)]
    pub fn new(identity: Identity, site: SiteKey, pass: Pass, body: Fut) -> Self {
        Self::branches(identity, site, pass, false, body)
    }

    /// A region whose body runs the branch written for the pass.
    ///
    /// `connected` says whether the body has a branch for the connected
    /// phase, which the initial phase then leaves markers for.
    #[doc(hidden)]
    pub fn branches(
        identity: Identity,
        site: SiteKey,
        pass: Pass,
        connected: bool,
        body: Fut,
    ) -> Self {
        Self {
            body,
            region: RegionId::new(identity, site),
            stash: None,
            pass,
            connecting: connected,
        }
    }
}

impl LiveView<Ready<Result<EmitToken>>> {
    /// Drives `view` inside a live body: the future an `emit!` awaits.
    ///
    /// The view's first content and every swap after it are handed to the
    /// enclosing poll as emissions, and the future resolves to the token
    /// once the view has no further updates.
    pub fn drive<V: View>(view: V) -> impl Future<Output = Result<EmitToken>> {
        DriveFuture::new(view).map_ok(|()| EmitToken)
    }
}

/// Frames `content` with the markers of `region`.
fn framed(region: RegionId, content: ViewHandle) -> ViewHandle {
    ViewBufferScope::with(|buffer| {
        buffer.block(|parts| {
            parts.push_region_start(region);
            parts.push_view_handle(content);
            parts.push_region_end(region);
        })
    })
}

impl<Fut> View for LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();

        match poll_body(this.body.as_mut(), cx) {
            (Poll::Pending, Some(Yield::First(first))) => {
                // Poll again to determine liveness. If the second poll returns pending, we
                // expect this view to yield again in the future.
                let (poll, yielded) = poll_body(this.body, cx);

                if let Poll::Ready(Err(e)) = poll {
                    return Poll::Ready(Err(e));
                }

                let streaming = poll.is_pending();
                // In a connected pass the region is already connected, so
                // nothing connects later.
                let connecting =
                    (*this.connecting || first.connecting) && *this.pass == Pass::Initial;
                if !streaming && !connecting {
                    // The body is done, so nothing will replace this content and it needs no
                    // markers.
                    return Poll::Ready(Ok(ViewFirst {
                        content: first.content,
                        streaming,
                        connecting,
                    }));
                }

                let region = *this.region;
                *this.stash = yielded.map(|yielded| yielded.into_swap(region));

                Poll::Ready(Ok(ViewFirst {
                    content: framed(region, first.content),
                    streaming,
                    connecting,
                }))
            }
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Pending, Some(Yield::Swap(_))) => {
                panic!("live view future yielded a swap before its first content")
            }
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

        if let Some(stash) = this.stash.take() {
            return Poll::Ready(Ok(Some(stash)));
        }

        let region = *this.region;

        match poll_body(this.body, cx) {
            (Poll::Pending, Some(yielded)) => Poll::Ready(Ok(Some(yielded.into_swap(region)))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(_)), None) => Poll::Ready(Ok(None)),
        }
    }
}
