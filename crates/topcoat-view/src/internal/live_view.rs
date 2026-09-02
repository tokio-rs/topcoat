use std::{
    future::Ready,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::TryFutureExt;
use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::yielder::{DriveFuture, Yield, poll_body};
use crate::{EmitToken, RegionId, View, ViewBufferScope, ViewFirst, ViewSwap};

pin_project! {
    /// A `live!` region as a [`View`]: a body future whose emissions become
    /// the region's content.
    ///
    /// The body reports each emission out of band while it runs. The first
    /// one becomes the view's first content; when the body is already done
    /// at that point the content is final and needs no markers. Otherwise
    /// the content is framed with the markers of a freshly allocated region
    /// and every later emission becomes a swap of that region.
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: Option<RegionId>,
        stash: Option<ViewSwap>,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self {
            body,
            region: None,
            stash: None,
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

                let live = poll.is_pending();
                if !live {
                    // The body is done, so nothing will replace this content and it needs no
                    // markers.
                    return Poll::Ready(Ok(ViewFirst {
                        content: first.content,
                        live,
                    }));
                }

                let region = *this.region.get_or_insert_with(RegionId::next);
                *this.stash = yielded.map(|yielded| yielded.into_swap(region));

                let first = ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|parts| {
                            parts.push_region_start(region);
                            parts.push_view_handle(first.content);
                            parts.push_region_end(region);
                        })
                    }),
                    live,
                };
                Poll::Ready(Ok(first))
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

        let region = (*this.region).expect("live view polled for a swap before its first content");

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
