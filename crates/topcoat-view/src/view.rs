use std::{
    future::poll_fn,
    ops::DerefMut,
    pin::{Pin, pin},
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{RegionId, buffer::ViewHandle};

/// A [`View`]'s first content, resolved from [`View::poll_first`].
#[derive(Debug)]
pub struct ViewFirst {
    /// The content, ready to render with the surrounding document.
    pub content: ViewHandle,
    /// Whether the view can still change the content through
    /// [`View::poll_swap`] after it went out.
    pub live: bool,
}

/// A replacement for a live region of content that already went out,
/// yielded by [`View::poll_swap`].
#[derive(Debug)]
pub struct ViewSwap {
    /// The region the replacement belongs to.
    pub region: RegionId,
    /// The content that replaces what the region currently shows.
    pub replacement: ViewHandle,
}

/// The value a `live!` body returns, as a reminder that it must emit content.
///
/// The `emit!` macro evaluates to a [`Result`] carrying this token, so a body
/// that ends with an emission returns the right type. A body that emits
/// earlier, for example inside a loop, can end with `Ok(EmitToken)` instead.
/// The token does not prove that anything was emitted; the body must still
/// emit at least once. See the
/// [`live!`](https://docs.rs/topcoat/latest/topcoat/view/macro.live.html)
/// documentation for details.
#[derive(Debug)]
pub struct EmitToken;

/// A piece of HTML that can keep changing while a response streams.
///
/// A view is polled in two phases. [`poll_first`](Self::poll_first)
/// resolves once, to the content that renders with the surrounding
/// document. If that content is live, [`poll_swap`](Self::poll_swap) then
/// yields replacements for regions of it until the view is done.
///
/// The `view!` and `live!` macros build implementations of this trait.
/// Application code composes those views and rarely implements the trait by
/// hand. To get the content of a view, use [`ViewExt::first`] or
/// [`ViewExt::single`].
pub trait View: Send {
    /// Resolves the view's first content.
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>>;

    /// Yields the next replacement for a region of the first content, or
    /// `None` when the view is done changing.
    ///
    /// Call this only after [`poll_first`](Self::poll_first) resolved to
    /// live content.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>>;
}

/// Methods available on every [`View`].
pub trait ViewExt: View {
    /// Resolves the view's first content and drops the view.
    ///
    /// If the view is live, the replacements it would stream later are
    /// never produced, and the content stays as it first resolved.
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

    /// Resolves the content of a view that is expected not to be live.
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

    /// Erases the view's concrete type behind a boxed one.
    ///
    /// Every `view!` invocation has its own anonymous type. A function that
    /// returns different views from different branches can box each of them
    /// to give them a common type.
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

/// A pinned pointer to a view, such as a [`BoxView`], polls the view it
/// points at.
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
