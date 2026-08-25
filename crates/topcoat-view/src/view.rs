use std::{
    future::poll_fn,
    pin::{Pin, pin},
    task::{Context, Poll},
};

use topcoat_core::{context::Cx, error::Result};

use crate::buffer::{ViewBuffer, ViewHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub(crate) u64);

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
    /// its first content are discarded.
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

