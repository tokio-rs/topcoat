use std::{
    future::poll_fn,
    ops::DerefMut,
    pin::{Pin, pin},
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{RegionId, buffer::ViewHandle};

#[derive(Debug)]
pub struct ViewFirst {
    pub content: ViewHandle,
    pub live: bool,
}

#[derive(Debug)]
pub struct ViewSwap {
    pub region: RegionId,
    pub replacement: ViewHandle,
}

#[derive(Debug)]
pub struct EmitToken;

pub trait View: Send {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>>;
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>>;
}

pub trait ViewExt: View {
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
