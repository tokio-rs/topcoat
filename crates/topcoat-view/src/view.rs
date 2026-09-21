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
    /// Whether the view still changes the content through
    /// [`View::poll_swap`] within the current response.
    pub streaming: bool,
    /// Whether the content holds a region with a connected phase, which
    /// changes the content in a later connect pass.
    pub connecting: bool,
}

impl ViewFirst {
    /// Whether the content stays as it resolved: the view neither streams
    /// within the current response nor connects afterwards.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        !self.streaming && !self.connecting
    }
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

/// The value a live region's body returns to show it emitted content.
///
/// The `emit!` macro evaluates to a [`Result`] carrying this token, and a
/// `live!` body returns one, so ending the body with an emission is the
/// natural way to satisfy the type. The `live!` guide describes the token
/// and how to construct one when the body does not end with an emission.
#[derive(Debug)]
pub struct EmitToken;

/// A piece of HTML that can keep changing while a response streams.
///
/// A view is polled in two phases. [`poll_first`](Self::poll_first)
/// resolves once, to the content that renders with the surrounding
/// document. When that content reports itself as streaming,
/// [`poll_swap`](Self::poll_swap) takes over and yields replacements for
/// regions of it until the view is done.
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
    /// streaming content.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>>;
}

/// Methods available on every [`View`].
pub trait ViewExt: View {
    /// Resolves the view's first content and discards the view.
    ///
    /// Replacements a streaming view would deliver afterwards never happen;
    /// the content stays as it first resolved.
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

    /// Resolves the content of a view that does not change after it went
    /// out.
    ///
    /// # Panics
    ///
    /// Panics if the view's first content is streaming or connecting.
    fn single(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            let first = poll_fn(|cx| view.as_mut().poll_first(cx)).await?;
            assert!(
                first.is_settled(),
                "used `.single()` on a View that changes after it went out"
            );
            Ok(first.content)
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

/// The empty view: renders nothing and never changes.
impl View for () {
    fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        Poll::Ready(Ok(ViewFirst {
            content: ViewHandle::empty(),
            streaming: false,
            connecting: false,
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
