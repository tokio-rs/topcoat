use std::{
    fmt,
    future::poll_fn,
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
};

use futures_core::Stream;
use topcoat_core::error::Result;

use crate::buffer::ViewHandle;

/// The identity of a live region within a rendered view.
///
/// Displays as the number that marks the region's boundaries in the HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub(crate) u64);

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A replacement for the content of a live region, emitted after a view's
/// first content resolved.
#[derive(Debug)]
pub struct Swap {
    /// The region whose content is replaced.
    pub region: RegionId,
    /// The region's new content, self-contained: it renders without the
    /// buffer the view's first content was built in.
    pub replacement: ViewHandle,
}

/// A lazy view: an inert value that builds its content when polled.
///
/// A `view!` invocation evaluates to a value implementing this trait, and a
/// component returns one as `Result<impl View>`. Constructing a view does no
/// work; everything it writes happens inside the poll methods, into the
/// [`ViewBuffer`](crate::ViewBuffer) the view was built with.
///
/// [`poll_first`](View::poll_first) drives the view to its first content: a
/// [`ViewHandle`] pointing at the instruction block the view appended to
/// its buffer. After that, [`poll_swap`](View::poll_swap) streams the
/// updates its live regions emit, until it returns `None`.
pub trait View: Send {
    /// Polls toward the view's first content.
    ///
    /// On `Ready`, the view has appended its instruction block to its buffer
    /// and the returned handle points at it. Must not be polled again after
    /// it returned `Ready`.
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>>;

    /// Polls for the next live update, once the first content resolved.
    ///
    /// Returns `Ready(None)` when the view emits no further updates. Must
    /// only be polled after [`poll_first`](View::poll_first) returned
    /// `Ready`.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>>;
}

/// Combinators available on every [`View`].
///
/// Blanket implemented, so implementing [`View`] is enough to get them and an
/// implementation never has to care about them.
///
/// The handle a combinator resolves is self-contained when the view builds
/// in a buffer of its own, as a `view!` invocation naming its context does;
/// a view built against a shared buffer resolves a handle into that buffer,
/// which [`ViewHandle::seal`] makes self-contained.
pub trait ViewExt: View {
    /// Resolves the view's first content.
    ///
    /// Any updates the view would emit after its first content are
    /// discarded; [`single`](ViewExt::single) asserts there are none
    /// instead.
    fn first(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            poll_fn(|cx| view.as_mut().poll_first(cx)).await
        }
    }

    /// Resolves the content of a view that never updates.
    ///
    /// Where [`first`](ViewExt::first) discards the updates a view emits
    /// after its first content, `single` asserts there are none: the view
    /// must complete right after its first content, without emitting or
    /// waiting on a swap. This is the method to reach for when a view is
    /// rendered once, into a fragment or a string.
    ///
    /// # Panics
    ///
    /// Panics if the view is live, that is, if it emits or waits on an
    /// update after its first content. Such a view is rendered with
    /// [`live`](ViewExt::live) instead.
    fn single(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            let content = poll_fn(|cx| view.as_mut().poll_first(cx)).await?;
            let mut cx = Context::from_waker(Waker::noop());
            match view.as_mut().poll_swap(&mut cx) {
                Poll::Ready(None) => Ok(content),
                Poll::Ready(Some(Err(error))) => Err(error),
                Poll::Ready(Some(Ok(_))) | Poll::Pending => panic!(
                    "`single` called on a live view, which updates after its first content; \
                     render it with `live` to receive the updates"
                ),
            }
        }
    }

    /// Resolves the view's first content and keeps the updates that follow.
    ///
    /// The stream beside the content yields a [`Swap`] for every live region
    /// that re-renders and ends once the view has no further updates.
    fn live(self) -> impl Future<Output = Result<(ViewHandle, Swaps<Self>)>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = Box::pin(self);
            let content = poll_fn(|cx| view.as_mut().poll_first(cx)).await?;
            Ok((content, Swaps { view }))
        }
    }

    /// Erases the view's concrete type behind a boxed one.
    ///
    /// Every `view!` invocation has its own anonymous type, so a function
    /// returning `impl View` from multiple `return` sites must box each view
    /// to give them a common type.
    fn boxed<'a>(self) -> BoxView<'a>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }
}

impl<V: View + ?Sized> ViewExt for V {}

/// The updates a view emits after its first content, returned by
/// [`ViewExt::live`].
///
/// Yields a [`Swap`] for every live region that re-renders and ends once the
/// view has no further updates.
pub struct Swaps<V> {
    view: Pin<Box<V>>,
}

impl<V: View> Stream for Swaps<V> {
    type Item = Result<Swap>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().view.as_mut().poll_swap(cx)
    }
}

impl View for () {
    fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        Poll::Ready(Ok(ViewHandle::empty()))
    }

    fn poll_swap(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        Poll::Ready(None)
    }
}

pub type BoxView<'a> = Pin<Box<dyn View + 'a>>;

impl View for BoxView<'_> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        self.get_mut().as_mut().poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        self.get_mut().as_mut().poll_swap(cx)
    }
}
