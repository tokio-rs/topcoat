use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::{Error, Result};

use crate::{RegionId, View, ViewBufferScope, ViewFirst, ViewSwap, internal::EmitView};

pin_project! {
    /// A [`View`] that replaces its child content with a fallback when any
    /// part of the child fails to render.
    ///
    /// The child renders in place. When its first content is settled, no
    /// region is created. Otherwise the content renders inside a live
    /// region, so a failure while it streams can still swap the fallback in
    /// over everything the child already sent.
    #[project = ErrorBoundaryViewProj]
    pub enum ErrorBoundaryView<C, F, V> {
        Child {
            #[pin]
            child: EmitView<C>,
            // Taken once, when the child fails.
            fallback: Option<F>,
            region: RegionId,
        },
        Fallback {
            #[pin]
            view: EmitView<V>,
        },
    }
}

impl<C, F, V> ErrorBoundaryView<C, F, V> {
    #[doc(hidden)]
    pub fn new(region: RegionId, fallback: F, child: C) -> Self {
        Self::Child {
            child: EmitView::new(region, child),
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
                    Ok(first) if first.is_settled() => return Poll::Ready(Ok(first)),
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
                        let view = EmitView::new(*region, view);
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
                        let view = EmitView::new(*region, view);
                        self.as_mut().set(Self::Fallback { view });
                    }
                },
                ErrorBoundaryViewProj::Fallback { view } => return view.poll_swap(cx),
            }
        }
    }
}
