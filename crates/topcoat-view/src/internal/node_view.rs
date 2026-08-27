use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    BoxView, Child, NodeViewParts, Step, View,
    buffer::ViewBufferScope,
    internal::{LiveView, MoveView, ScopeView},
};

pin_project! {
    /// A node position's value as a [`View`].
    ///
    /// The `view!` expansion wraps every dynamic node position in this
    /// type, with the request context of the template. A [`NodeViewParts`]
    /// value renders as a one-shot view, appending its parts to the buffer
    /// of the build in a single burst and never updating; a nested view
    /// polls through in place.
    ///
    /// A fully generic `T: View` impl would overlap the [`NodeViewParts`]
    /// one, so the nested-view side is implemented per wrapped type instead.
    pub struct NodeView<'a, T> {
        cx: &'a Cx,
        #[pin]
        value: Option<T>,
    }
}

impl<'a, T> NodeView<'a, T> {
    #[must_use]
    pub fn new(cx: &'a Cx, value: T) -> Self {
        Self {
            cx,
            value: Some(value),
        }
    }

    /// Projects to the wrapped view, for the nested-view impls.
    fn nested(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project()
            .value
            .as_pin_mut()
            .expect("a nested view keeps its value")
    }
}

/// A [`NodeViewParts`] value: its parts are appended in one burst, and the
/// position never updates.
impl<T> View for NodeView<'_, T>
where
    T: NodeViewParts + Send + Unpin,
{
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        let value = this
            .value
            .get_mut()
            .take()
            .expect("`poll` called again after the position's content resolved");
        let content = ViewBufferScope::block(|parts| value.into_view_parts(this.cx, parts));
        Poll::Ready(Ok(Step::Content {
            content,
            live: false,
        }))
    }
}

/// A component's child content: the children's view polls through in place.
impl View for NodeView<'_, Child<'_>> {
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        let child = this
            .value
            .get_mut()
            .as_mut()
            .expect("a nested view keeps its value");
        child.view().poll(cx)
    }
}

/// A boxed view: it polls through in place.
impl View for NodeView<'_, BoxView<'_>> {
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        self.nested().get_mut().as_mut().poll(cx)
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
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
                self.nested().poll(cx)
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

nested_view! {
    /// A nested `view!` invocation taking part in the build: it polls
    /// through in place.
    impl<V> for ScopeView<V>
}
