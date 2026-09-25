use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::{Error, Result};

use crate::{
    RegionId, View, ViewBufferScope, ViewFirst, ViewSwap,
    internal::{EmitView, ScopeView},
};

pin_project! {
    /// A [`View`] that replaces its child content with a fallback when any
    /// part of the child fails to render.
    ///
    /// A child with no live updates renders directly. A live child renders
    /// in a region so a later failure can replace all its content.
    ///
    /// The fallback closure runs once on the first child error. When it
    /// returns a view, the failed child is dropped and that view takes over,
    /// including any updates of its own. Errors from the fallback propagate.
    #[project = ErrorBoundaryViewProj]
    pub enum ErrorBoundaryView<C, F, V> {
        /// The child is rendering and the fallback has not been used.
        Child {
            #[pin]
            child: C,
            // Taken once, when the child fails.
            fallback: Option<F>,
            region: RegionId,
        },
        /// The child failed and its fallback is rendering in its place.
        Fallback {
            #[pin]
            view: EmitView<ScopeView<V>>,
        },
    }
}

impl<C, F, V> ErrorBoundaryView<C, F, V> {
    /// Guards `child`, replacing `region` with `fallback` after a late error.
    ///
    /// An error before first content resolves renders the fallback in place.
    #[doc(hidden)]
    pub fn new(region: RegionId, fallback: F, child: C) -> Self {
        Self::Child {
            child,
            fallback: Some(fallback),
            region,
        }
    }
}

impl<C, F, V> View for ErrorBoundaryView<C, F, V>
where
    C: View,
    F: FnOnce(Error) -> Result<V> + Send,
    V: View,
{
    fn poll_first(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        loop {
            match self.as_mut().project() {
                ErrorBoundaryViewProj::Child {
                    child,
                    fallback,
                    region,
                } => match ready!(child.poll_first(cx)) {
                    // A settled child cannot fail anymore, so it needs no
                    // region to swap the fallback into.
                    Ok(first) if !first.live => return Poll::Ready(Ok(first)),
                    Ok(first) => {
                        let content = ViewBufferScope::with(|buffer| {
                            buffer.block(|parts| {
                                parts.push_region_start(*region);
                                parts.push_view_handle(first.content);
                                parts.push_region_end(*region);
                            })
                        });
                        return Poll::Ready(Ok(ViewFirst { content, ..first }));
                    }
                    Err(error) => {
                        let view = fallback.take().expect("the fallback runs once")(error)?;
                        let view = EmitView::new(*region, ScopeView::new(view));
                        self.as_mut().set(Self::Fallback { view });
                    }
                },
                ErrorBoundaryViewProj::Fallback { view } => return view.poll_first(cx),
            }
        }
    }

    fn poll_swap(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        loop {
            match self.as_mut().project() {
                ErrorBoundaryViewProj::Child {
                    child,
                    fallback,
                    region,
                } => match ready!(child.poll_swap(cx)) {
                    Ok(swap) => return Poll::Ready(Ok(swap)),
                    // The fallback replaces everything the child sent.
                    Err(error) => {
                        let view = fallback.take().expect("the fallback runs once")(error)?;
                        let view = ScopeView::self_contained(|| view);
                        let view = EmitView::new(*region, view);
                        self.as_mut().set(Self::Fallback { view });
                    }
                },
                ErrorBoundaryViewProj::Fallback { view } => return view.poll_swap(cx),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        pin::pin,
        sync::atomic::{AtomicBool, Ordering},
        task::Waker,
    };

    use topcoat_core::{
        context::Cx,
        identity::{Identity, SiteKey},
    };

    use super::*;
    use crate::ViewHandle;

    struct LiveChild<'a> {
        swap: Option<Result<ViewSwap>>,
        dropped: &'a AtomicBool,
    }

    impl View for LiveChild<'_> {
        fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
            Poll::Ready(Ok(ViewFirst {
                content: ViewHandle::empty(),
                live: true,
            }))
        }

        fn poll_swap(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Option<ViewSwap>>> {
            Poll::Ready(self.get_mut().swap.take().transpose())
        }
    }

    impl Drop for LiveChild<'_> {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::Relaxed);
        }
    }

    fn region(ordinal: u32) -> RegionId {
        RegionId::new(
            Identity::ROOT,
            SiteKey::new(file!(), line!(), column!(), ordinal),
        )
    }

    #[test]
    fn swaps_pass_through_after_first_content() {
        let dropped = AtomicBool::new(false);
        let child = LiveChild {
            swap: Some(Ok(ViewSwap {
                region: region(1),
                replacement: ViewHandle::unescaped_unchecked("<p>updated</p>"),
            })),
            dropped: &dropped,
        };
        let mut view = pin!(ScopeView::new(ErrorBoundaryView::new(
            region(0),
            |_| Ok(()),
            child,
        )));
        let mut cx = Context::from_waker(Waker::noop());

        let Poll::Ready(Ok(first)) = view.as_mut().poll_first(&mut cx) else {
            panic!("expected the child's first content");
        };
        assert!(first.live);

        let Poll::Ready(Ok(Some(swap))) = view.as_mut().poll_swap(&mut cx) else {
            panic!("expected the child's swap");
        };
        assert_eq!(swap.region, region(1));
        assert_eq!(swap.replacement.render(&Cx::default()), "<p>updated</p>");
        assert!(!dropped.load(Ordering::Relaxed));
        assert!(matches!(
            view.as_mut().poll_swap(&mut cx),
            Poll::Ready(Ok(None))
        ));
    }

    #[test]
    fn a_swap_error_drops_the_child_and_replaces_the_boundary() {
        let dropped = AtomicBool::new(false);
        let child = LiveChild {
            swap: Some(Err(io::Error::other("late failure").into())),
            dropped: &dropped,
        };
        let mut view = pin!(ScopeView::new(ErrorBoundaryView::new(
            region(0),
            |error: Error| {
                assert_eq!(error.to_string(), "late failure");
                Ok(())
            },
            child,
        )));
        let mut cx = Context::from_waker(Waker::noop());

        let Poll::Ready(Ok(first)) = view.as_mut().poll_first(&mut cx) else {
            panic!("expected the child's first content");
        };
        assert!(first.live);

        let Poll::Ready(Ok(Some(swap))) = view.as_mut().poll_swap(&mut cx) else {
            panic!("expected the fallback to replace the boundary");
        };
        assert_eq!(swap.region, region(0));
        assert!(swap.replacement.is_empty());
        assert!(dropped.load(Ordering::Relaxed));
        assert!(matches!(
            view.as_mut().poll_swap(&mut cx),
            Poll::Ready(Ok(None))
        ));
    }

    #[test]
    fn a_settled_child_needs_no_markers_or_swaps() {
        let mut view = pin!(ErrorBoundaryView::new(region(0), |_| Ok(()), ()));
        let mut cx = Context::from_waker(Waker::noop());

        let Poll::Ready(Ok(first)) = view.as_mut().poll_first(&mut cx) else {
            panic!("expected settled content");
        };
        assert!(!first.live);
        assert!(first.content.is_empty());
    }
}
