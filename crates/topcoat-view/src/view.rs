use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
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

pin_project! {
    #[project = ThenViewProj]
    pub(crate) enum ThenView<F, V> {
        Future { #[pin] future: F },
        View { #[pin] view: V },
    }
}

impl<F, V> ThenView<F, V>
where
    F: Future<Output = Result<V>>,
{
    #[must_use]
    pub fn new(future: F) -> Self {
        Self::Future { future }
    }
}

impl<F, V> View for ThenView<F, V>
where
    F: Future<Output = Result<V>> + Send,
    V: View,
{
    fn poll_first(
        mut self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        loop {
            match self.as_mut().project() {
                ThenViewProj::Future { future } => {
                    let view = ready!(future.poll(task))?;
                    self.as_mut().set(Self::View { view });
                }
                ThenViewProj::View { view } => return view.poll_first(cx, task, buf),
            }
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        match self.project() {
            ThenViewProj::Future { .. } => {
                panic!("`poll_swap` called before `poll_first` returned `Ready`")
            }
            ThenViewProj::View { view } => view.poll_swap(cx, task, buf),
        }
    }
}
