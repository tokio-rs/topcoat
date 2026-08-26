use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    BoxView, Child, NodeViewParts, Swap, View,
    buffer::{ViewBuffer, ViewHandle},
    internal::{LiveView, MoveView},
};

pin_project! {
    /// A node position's value as a [`View`].
    ///
    /// The `view!` expansion wraps every dynamic node position in this
    /// type. A [`NodeViewParts`] value renders as a one-shot view, appending
    /// its parts in a single burst and never updating; a nested view polls
    /// through in place.
    ///
    /// A fully generic `T: View` impl would overlap the [`NodeViewParts`]
    /// one, so the nested-view side is implemented per wrapped type instead.
    pub struct NodeView<T> {
        #[pin]
        value: Option<T>,
    }
}

impl<T> NodeView<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
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
impl<T> View for NodeView<T>
where
    T: NodeViewParts + Send + Unpin,
{
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        _task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let value = self.project().value.get_mut().take().expect(FIRST_AGAIN);
        let view = buf.block(cx, |b| value.into_view_parts(cx, b.parts()));
        Poll::Ready(Ok(view))
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

/// A component's child content: the children's view polls through in place.
impl View for NodeView<Child<'_>> {
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        self.nested(FIRST_AGAIN)
            .get_mut()
            .view
            .as_mut()
            .poll_first(cx, task, buf)
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        self.nested(SWAP_BEFORE_FIRST)
            .get_mut()
            .view
            .as_mut()
            .poll_swap(cx, task, buf)
    }
}

/// A boxed view: it polls through in place.
impl View for NodeView<BoxView<'_>> {
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        self.nested(FIRST_AGAIN)
            .get_mut()
            .as_mut()
            .poll_first(cx, task, buf)
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        self.nested(SWAP_BEFORE_FIRST)
            .get_mut()
            .as_mut()
            .poll_swap(cx, task, buf)
    }
}

/// Implements the nested-view side of [`NodeView`] for a view type that is
/// pinned in place: the wrapped view polls through untouched.
macro_rules! nested_view {
    ($(#[$doc:meta])* impl<$($param:ident),*> for $ty:ty) => {
        $(#[$doc])*
        impl<$($param),*> View for NodeView<$ty>
        where
            $ty: View,
        {
            fn poll_first(
                self: Pin<&mut Self>,
                cx: &Cx,
                task: &mut Context<'_>,
                buf: &mut ViewBuffer,
            ) -> Poll<Result<ViewHandle>> {
                self.nested(FIRST_AGAIN).poll_first(cx, task, buf)
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                cx: &Cx,
                task: &mut Context<'_>,
                buf: &mut ViewBuffer,
            ) -> Poll<Option<Result<Swap>>> {
                self.nested(SWAP_BEFORE_FIRST).poll_swap(cx, task, buf)
            }
        }
    };
}

nested_view! {
    /// A `live!` region: it polls through in place.
    impl<Fut> for LiveView<Fut>
}

nested_view! {
    /// A nested `view!` invocation: it polls through in place.
    impl<Fut> for MoveView<Fut>
}
