use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    BoxView, Child, NodeViewParts, Swap, View, ViewBuffer,
    buffer::ViewHandle,
    internal::{LiveView, MoveView},
};

pin_project! {
    /// A node position's value as a [`View`].
    ///
    /// The `view!` expansion wraps every dynamic node position in this
    /// type, with the request context and buffer of the template. A
    /// [`NodeViewParts`] value renders as a one-shot view, appending its
    /// parts to the buffer in a single burst and never updating; a nested
    /// view polls through in place.
    ///
    /// A fully generic `T: View` impl would overlap the [`NodeViewParts`]
    /// one, so the nested-view side is implemented per wrapped type instead.
    pub struct NodeView<'a, T> {
        cx: &'a Cx,
        buf: &'a ViewBuffer,
        #[pin]
        value: Option<T>,
    }
}

impl<'a, T> NodeView<'a, T> {
    #[must_use]
    pub fn new(cx: &'a Cx, buf: &'a ViewBuffer, value: T) -> Self {
        Self {
            cx,
            buf,
            value: Some(value),
        }
    }

    /// Projects to the wrapped view, for the nested-view impls.
    fn nested(self: Pin<&mut Self>, expect: &'static str) -> Pin<&mut T> {
        self.project().value.as_pin_mut().expect(expect)
    }
}

const FIRST_AGAIN: &str = "`poll_first` called again after it returned `Ready`";
const SWAP_BEFORE_FIRST: &str = "`poll_swap` called before `poll_first` returned `Ready`";

/// A [`NodeViewParts`] value: its parts are appended in one burst, and the
/// position never updates.
impl<T> View for NodeView<'_, T>
where
    T: NodeViewParts + Send + Unpin,
{
    fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let value = this.value.get_mut().take().expect(FIRST_AGAIN);
        let view = this
            .buf
            .block(|parts| value.into_view_parts(this.cx, parts));
        Poll::Ready(Ok(view))
    }

    fn poll_swap(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        Poll::Ready(None)
    }
}

/// A component's child content: the children's view polls through in place,
/// built against this position's context and buffer if it was deferred.
impl View for NodeView<'_, Child<'_>> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let child = this.value.get_mut().as_mut().expect(FIRST_AGAIN);
        child.view(this.cx, this.buf).poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        let child = this.value.get_mut().as_mut().expect(SWAP_BEFORE_FIRST);
        child.view(this.cx, this.buf).poll_swap(cx)
    }
}

/// A boxed view: it polls through in place.
impl View for NodeView<'_, BoxView<'_>> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        self.nested(FIRST_AGAIN).get_mut().as_mut().poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        self.nested(SWAP_BEFORE_FIRST)
            .get_mut()
            .as_mut()
            .poll_swap(cx)
    }
}

/// Implements the nested-view side of [`NodeView`] for a view type that is
/// pinned in place: the wrapped view polls through untouched.
macro_rules! nested_view {
    ($(#[$doc:meta])* impl<$($param:tt),*> for $ty:ty) => {
        $(#[$doc])*
        impl<$($param),*> View for NodeView<'_, $ty>
        where
            $ty: View,
        {
            fn poll_first(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Result<ViewHandle>> {
                self.nested(FIRST_AGAIN).poll_first(cx)
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Option<Result<Swap>>> {
                self.nested(SWAP_BEFORE_FIRST).poll_swap(cx)
            }
        }
    };
}

nested_view! {
    /// A `live!` region: it polls through in place.
    impl<'b, Fut> for LiveView<'b, Fut>
}

nested_view! {
    /// A nested `view!` invocation: it polls through in place.
    impl<Fut> for MoveView<Fut>
}
