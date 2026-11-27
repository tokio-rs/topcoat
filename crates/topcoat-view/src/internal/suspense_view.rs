use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{RegionId, View, ViewFirst, ViewScope, ViewSwap};

pin_project! {
    /// A [`View`] that shows a fallback until its child content is ready.
    ///
    /// Polls the child first. If its initial content is ready, it renders
    /// directly. Otherwise, a live region shows the fallback until the
    /// child's content replaces it.
    ///
    /// With `wait` enabled, the boundary waits for the child's first content
    /// and renders it in place. It never polls the fallback or creates a
    /// region of its own.
    ///
    /// The fallback can stream updates while the child is pending. Once the
    /// child resolves, only its updates pass through. Errors from either view
    /// propagate to the caller.
    pub struct SuspenseView<F, C> {
        #[pin]
        fallback: F,
        #[pin]
        child: C,
        // The child can finish after the surrounding first content has
        // gone out, so it is a build of its own with its own scope.
        child_scope: ViewScope,
        region: RegionId,
        wait: bool,
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
    Fallback { live: bool },
    /// The child's first content went out and its swaps pass through.
    Child,
    /// The child has no more updates.
    Done,
}

impl<F, C> SuspenseView<F, C> {
    /// Creates a boundary whose fallback is replaced at `region` when needed.
    ///
    /// Set `wait` to delay the boundary's first content until the child is
    /// ready, without showing the fallback.
    #[doc(hidden)]
    pub fn new(region: RegionId, fallback: F, child: C, wait: bool) -> Self {
        Self {
            fallback,
            child,
            child_scope: ViewScope::new(),
            region,
            wait,
            state: State::Start,
        }
    }
}

impl<F, C> View for SuspenseView<F, C>
where
    F: View,
    C: View,
{
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        scope: &mut ViewScope,
    ) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        assert!(
            *this.state == State::Start,
            "polled a suspense view's first content after it resolved"
        );

        match scope.nested(this.child_scope, |inner| this.child.poll_first(cx, inner)) {
            Poll::Ready(Ok(first)) => {
                *this.state = if first.live {
                    State::Child
                } else {
                    State::Done
                };
                return Poll::Ready(Ok(ViewFirst {
                    content: this.child_scope.seal(first.content),
                    live: first.live,
                }));
            }
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending if *this.wait => return Poll::Pending,
            Poll::Pending => {}
        }

        match this.fallback.poll_first(cx, scope) {
            Poll::Ready(Ok(first)) => {
                *this.state = State::Fallback { live: first.live };
                let content = scope.block(|parts| {
                    parts.push_region_start(*this.region);
                    parts.push_view_handle(first.content);
                    parts.push_region_end(*this.region);
                });
                Poll::Ready(Ok(ViewFirst {
                    content,
                    live: true,
                }))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        scope: &mut ViewScope,
    ) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        let live = match *this.state {
            State::Start => {
                panic!("polled a suspense view for swaps before its first content resolved")
            }
            State::Child => {
                return match scope.nested(this.child_scope, |inner| this.child.poll_swap(cx, inner))
                {
                    Poll::Ready(Ok(None)) => {
                        *this.state = State::Done;
                        Poll::Ready(Ok(None))
                    }
                    Poll::Ready(Ok(Some(swap))) => Poll::Ready(Ok(Some(ViewSwap {
                        region: swap.region,
                        replacement: this.child_scope.seal(swap.replacement),
                    }))),
                    poll => poll,
                };
            }
            State::Done => return Poll::Ready(Ok(None)),
            State::Fallback { live } => live,
        };

        // Only replace the suspense region when its fallback went out.
        match scope.nested(this.child_scope, |inner| this.child.poll_first(cx, inner)) {
            Poll::Ready(Ok(first)) => {
                *this.state = if first.live {
                    State::Child
                } else {
                    State::Done
                };
                return Poll::Ready(Ok(Some(ViewSwap {
                    region: *this.region,
                    replacement: this.child_scope.seal(first.content),
                })));
            }
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => {}
        }

        // The child is still pending, so the fallback can keep streaming.
        if live {
            match this.fallback.poll_swap(cx, scope) {
                Poll::Ready(Ok(None)) => {
                    *this.state = State::Fallback { live: false };
                    Poll::Pending
                }
                swap => swap,
            }
        } else {
            Poll::Pending
        }
    }
}
