use std::{
    future::poll_fn,
    ops::DerefMut,
    pin::{Pin, pin},
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{RegionId, buffer::ViewHandle};

/// The initial content returned by [`View::poll_first`].
#[derive(Debug)]
pub struct ViewFirst {
    /// The content, ready to render with the surrounding document.
    pub content: ViewHandle,
    /// Whether [`View::poll_swap`] can produce updates to this content.
    pub live: bool,
}

/// An update to a live region, returned by [`View::poll_swap`].
#[derive(Debug)]
pub struct ViewSwap {
    /// The region the replacement belongs to.
    pub region: RegionId,
    /// The content that replaces what the region currently shows.
    pub replacement: ViewHandle,
}

/// The return token for a live region's body.
///
/// `emit!` returns a [`Result`] containing this token, so a `live!` body can
/// end with an emission. Constructing the token directly does not emit
/// content. The body must still emit at least once.
#[derive(Debug)]
pub struct EmitToken;

/// A piece of HTML that can keep changing while a response streams.
///
/// First, [`poll_first`](Self::poll_first) returns the initial content.
/// If it is live, call [`poll_swap`](Self::poll_swap) for region updates
/// until it returns `None`.
///
/// The `view!` and `live!` macros build implementations of this trait;
/// application code composes those rather than implementing it by hand.
pub trait View: Send {
    /// Resolves the view's first content.
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>>;

    /// Yields the next replacement for a region of the first content, or
    /// `None` when the view is done changing.
    ///
    /// Only meaningful after [`poll_first`](Self::poll_first) resolved to
    /// live content.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>>;
}

/// Methods available on every [`View`].
pub trait ViewExt: View {
    /// Resolves the view's first content and discards the view.
    ///
    /// Later updates from a live view are discarded.
    fn first(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            let first = poll_fn(|cx| view.as_mut().poll_first(cx)).await?;
            Ok(first.content)
        }
    }

    /// Resolves the content of a view that has no live updates.
    ///
    /// # Panics
    ///
    /// Panics if the view's first content is live.
    fn single(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            let first = poll_fn(|cx| view.as_mut().poll_first(cx)).await?;
            assert!(!first.live, "used `.single()` on a View that is live");
            Ok(first.content)
        }
    }

    /// Boxes the view to give different view types a common return type.
    ///
    /// Use this when a function returns different `view!` expressions, or
    /// when a recursive component needs a view type with a known size.
    fn boxed<'a>(self) -> BoxView<'a>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }
}

impl<V: View + ?Sized> ViewExt for V {}

/// The empty view: renders nothing and never changes.
impl View for () {
    fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        Poll::Ready(Ok(ViewFirst {
            content: ViewHandle::empty(),
            live: false,
        }))
    }

    fn poll_swap(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        Poll::Ready(Ok(None))
    }
}

/// A [`View`] with its concrete type erased, built with [`ViewExt::boxed`].
pub type BoxView<'a> = Pin<Box<dyn View + 'a>>;

/// A pinned pointer to a view, like a [`BoxView`], polls the view it points at.
impl<P> View for Pin<P>
where
    P: DerefMut + Unpin + Send,
    P::Target: View,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        self.get_mut().as_mut().poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        self.get_mut().as_mut().poll_swap(cx)
    }
}
