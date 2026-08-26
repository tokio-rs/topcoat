use std::{
    fmt,
    future::poll_fn,
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
};

use futures_core::Stream;
use topcoat_core::{context::Cx, error::Result};

use crate::buffer::{ViewBuffer, ViewHandle};

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
/// buffer the caller passes in.
///
/// [`poll_first`](View::poll_first) drives the view to its first content: a
/// [`ViewHandle`] pointing at the instruction block the view appended to
/// `buf`. After that, [`poll_swap`](View::poll_swap) streams the updates its
/// live regions emit, until it returns `None`.
pub trait View: Send {
    /// Polls toward the view's first content.
    ///
    /// On `Ready`, the view has appended its instruction block to `buf` and
    /// the returned handle points at it. Must not be polled again after it
    /// returned `Ready`.
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>>;

    /// Polls for the next live update, once the first content resolved.
    ///
    /// Returns `Ready(None)` when the view emits no further updates. Must
    /// only be polled after [`poll_first`](View::poll_first) returned
    /// `Ready`.
    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>>;
}

/// Combinators available on every [`View`].
///
/// Blanket implemented, so implementing [`View`] is enough to get them and an
/// implementation never has to care about them.
pub trait ViewExt: View {
    /// Resolves the view's first content.
    ///
    /// The returned handle is self-contained: it can be rendered, stored,
    /// or spliced into another view. Any updates the view would emit after
    /// its first content are discarded; [`single`](ViewExt::single) asserts
    /// there are none instead.
    fn first(self, cx: &Cx) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut buffer = ViewBuffer::new();
            let mut view = pin!(self);
            let content = poll_fn(|task| view.as_mut().poll_first(cx, task, &mut buffer)).await?;
            Ok(content.seal(buffer))
        }
    }

    /// Resolves the content of a view that never updates.
    ///
    /// The returned handle is self-contained, like the one
    /// [`first`](ViewExt::first) returns. Where `first` discards the updates
    /// a view emits after its first content, `single` asserts there are
    /// none: the view must complete right after its first content, without
    /// emitting or waiting on a swap. This is the method to reach for when
    /// a view is rendered once, into a fragment or a string.
    ///
    /// # Panics
    ///
    /// Panics if the view is live, that is, if it emits or waits on an
    /// update after its first content. Such a view is rendered with
    /// [`live`](ViewExt::live) instead.
    fn single(self, cx: &Cx) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut buffer = ViewBuffer::new();
            let mut view = pin!(self);
            let content = poll_fn(|task| view.as_mut().poll_first(cx, task, &mut buffer)).await?;
            let mut task = Context::from_waker(Waker::noop());
            match view
                .as_mut()
                .poll_swap(cx, &mut task, &mut ViewBuffer::new())
            {
                Poll::Ready(None) => Ok(content.seal(buffer)),
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
    /// The returned handle is self-contained, like the one
    /// [`first`](ViewExt::first) returns. The stream beside it yields a
    /// [`Swap`] for every live region that re-renders and ends once the view
    /// has no further updates.
    fn live(self, cx: &Cx) -> impl Future<Output = Result<(ViewHandle, Swaps<Self>)>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut buffer = ViewBuffer::new();
            let mut view = Box::pin(self);
            let content = poll_fn(|task| view.as_mut().poll_first(cx, task, &mut buffer)).await?;
            let swaps = Swaps {
                cx: cx.clone(),
                view,
                buffer: ViewBuffer::new(),
            };
            Ok((content.seal(buffer), swaps))
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
    cx: Cx,
    view: Pin<Box<V>>,
    buffer: ViewBuffer,
}

impl<V: View> Stream for Swaps<V> {
    type Item = Result<Swap>;

    fn poll_next(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        this.view
            .as_mut()
            .poll_swap(&this.cx, task, &mut this.buffer)
    }
}

impl View for () {
    fn poll_first(
        self: Pin<&mut Self>,
        _cx: &Cx,
        _task: &mut Context<'_>,
        _buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        Poll::Ready(Ok(ViewHandle::empty()))
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        _cx: &Cx,
        _task: &mut Context<'_>,
        _buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        Poll::Ready(None)
    }
}

pub type BoxView<'a> = Pin<Box<dyn View + 'a>>;

impl View for BoxView<'_> {
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        self.get_mut().as_mut().poll_first(cx, task, buf)
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        self.get_mut().as_mut().poll_swap(cx, task, buf)
    }
}
