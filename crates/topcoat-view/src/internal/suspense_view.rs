use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{RegionId, View, ViewBufferScope, ViewFirst, ViewSwap};

pin_project! {
    /// A [`View`] that shows a fallback until its child content is ready.
    ///
    /// The child is polled first. When its first content is ready right
    /// away, it renders in place and no region is created. Otherwise the
    /// fallback renders inside a live region and the child's content swaps
    /// into it once it resolves.
    ///
    /// When polled for swaps first, forwards the child's swaps without
    /// rendering first content or the fallback.
    pub struct SuspenseView<F, C> {
        #[pin]
        fallback: F,
        #[pin]
        child: C,
        region: RegionId,
        state: State,
    }
}

/// What a [`SuspenseView`] has shown so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nothing has resolved yet.
    Start,
    /// The fallback went out and the child is still pending. The flag says
    /// whether the fallback still yields swaps of its own.
    Fallback { streaming: bool },
    /// The child's swaps pass through, either after its first content went
    /// out or because only swaps were requested.
    Child,
}

impl<F, C> SuspenseView<F, C> {
    #[doc(hidden)]
    pub fn new(region: RegionId, fallback: F, child: C) -> Self {
        Self {
            fallback,
            child,
            region,
            state: State::Start,
        }
    }
}

impl<F, C> View for SuspenseView<F, C>
where
    F: View,
    C: View,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        assert!(
            *this.state == State::Start,
            "polled a suspense view's first content after it resolved"
        );

        match this.child.poll_first(cx) {
            Poll::Ready(Ok(first)) => {
                *this.state = State::Child;
                return Poll::Ready(Ok(first));
            }
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => {}
        }

        match this.fallback.poll_first(cx) {
            Poll::Ready(Ok(first)) => {
                *this.state = State::Fallback {
                    streaming: first.streaming,
                };
                let content = ViewBufferScope::with(|buffer| {
                    buffer.block(|parts| {
                        parts.push_region_start(*this.region);
                        parts.push_view_handle(first.content);
                        parts.push_region_end(*this.region);
                    })
                });
                Poll::Ready(Ok(ViewFirst {
                    content,
                    streaming: true,
                    connecting: first.connecting,
                }))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        let State::Fallback { streaming } = *this.state else {
            *this.state = State::Child;
            return this.child.poll_swap(cx);
        };

        // Only replace the suspense region when its fallback went out.
        match this.child.poll_first(cx) {
            Poll::Ready(Ok(first)) => {
                *this.state = State::Child;
                return Poll::Ready(Ok(Some(ViewSwap {
                    region: *this.region,
                    replacement: first.content,
                })));
            }
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => {}
        }

        // The child is still pending, so the fallback can keep streaming.
        if streaming {
            match this.fallback.poll_swap(cx) {
                Poll::Ready(Ok(None)) => {
                    *this.state = State::Fallback { streaming: false };
                    Poll::Pending
                }
                swap => swap,
            }
        } else {
            Poll::Pending
        }
    }
}
