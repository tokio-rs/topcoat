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
    /// The region holds the body of the phase it was built in. The body
    /// reports each emission out of band while it runs. The first one
    /// becomes the view's first content; when the body is already done at
    /// that point the content is final and needs no markers. Otherwise the
    /// content is framed with the markers of its stable region and every
    /// later emission becomes a swap of that region.
    pub struct LiveView<I, C> {
        #[pin]
        phase: Phase<I, C>,
        region: RegionId,
        stash: Option<ViewSwap>,
        // Whether the region has a connected phase to run after the page
        // has loaded.
        connecting: bool,
    }
}

pin_project! {
    /// The phase a [`LiveView`] runs, with the body written for it.
    #[project = PhaseProj]
    enum Phase<I, C> {
        /// The initial phase, run while the response is produced.
        Initial { #[pin] body: Option<I> },
        /// The connected phase, run once the page is connected.
        Connected { #[pin] body: Option<C> },
    }
}

impl<I, C> Phase<I, C>
where
    I: Future<Output = Result<EmitToken>> + Send,
    C: Future<Output = Result<EmitToken>> + Send,
{
    /// Polls the phase's body once, or returns `None` when the phase has no
    /// body.
    #[allow(clippy::type_complexity)]
    fn poll_body(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Option<(Poll<Result<EmitToken>>, Option<Yield>)> {
        match self.project() {
            PhaseProj::Initial { body } => body.as_pin_mut().map(|body| poll_body(body, cx)),
            PhaseProj::Connected { body } => body.as_pin_mut().map(|body| poll_body(body, cx)),
        }
    }
}

impl<Fut> LiveView<Fut, Fut>
where
    Fut: Future<Output = Result<EmitToken>> + Send,
{
    /// A region with one body serving both phases, run in `pass`.
    #[doc(hidden)]
    pub fn new(identity: Identity, site: SiteKey, pass: Pass, body: Fut) -> Self {
        match pass {
            Pass::Initial => Self::initial(identity, site, Some(body), false),
            Pass::Connected => Self::connected(identity, site, Some(body)),
        }
    }
}

impl<I, C> LiveView<I, C>
where
    I: Future<Output = Result<EmitToken>> + Send,
    C: Future<Output = Result<EmitToken>> + Send,
{
    /// A region in its initial phase, with the body written for it.
    ///
    /// `connecting` says whether the region has a connected phase to run
    /// later, which a region without an initial body renders as empty
    /// content between its markers.
    #[doc(hidden)]
    pub fn initial(identity: Identity, site: SiteKey, body: Option<I>, connecting: bool) -> Self {
        Self {
            phase: Phase::Initial { body },
            region: RegionId::new(identity, site),
            stash: None,
            connecting,
        }
    }

    /// A region in its connected phase, with the body written for it.
    #[doc(hidden)]
    pub fn connected(identity: Identity, site: SiteKey, body: Option<C>) -> Self {
        Self {
            phase: Phase::Connected { body },
            region: RegionId::new(identity, site),
            stash: None,
            connecting: false,
        }
    }
}

impl LiveView<Ready<Result<EmitToken>>, Ready<Result<EmitToken>>> {
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

impl<I, C> View for LiveView<I, C>
where
    I: Future<Output = Result<EmitToken>> + Send,
    C: Future<Output = Result<EmitToken>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();
        let region = *this.region;
        let connecting = *this.connecting;

        let Some(polled) = this.phase.as_mut().poll_body(cx) else {
            // No body for this phase: the region is empty until its other
            // phase fills it, which needs the markers.
            let content = if connecting {
                framed(region, ViewHandle::empty())
            } else {
                ViewHandle::empty()
            };
            return Poll::Ready(Ok(ViewFirst {
                content,
                streaming: false,
                connecting,
            }));
        };

        match polled {
            (Poll::Pending, Some(Yield::First(first))) => {
                // Poll again to determine liveness. If the second poll returns pending, we
                // expect this view to yield again in the future.
                let (poll, yielded) = this
                    .phase
                    .poll_body(cx)
                    .expect("the phase had a body on the poll before");

                if let Poll::Ready(Err(e)) = poll {
                    return Poll::Ready(Err(e));
                }

                let streaming = poll.is_pending();
                let connecting = connecting || first.connecting;
                if !streaming && !connecting {
                    // The body is done, so nothing will replace this content and it needs no
                    // markers.
                    return Poll::Ready(Ok(ViewFirst {
                        content: first.content,
                        streaming,
                        connecting,
                    }));
                }

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
        let Some(polled) = this.phase.poll_body(cx) else {
            return Poll::Ready(Ok(None));
        };

        match polled {
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
