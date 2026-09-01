use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{
    BoxView, View, ViewFirst, ViewSwap,
    internal::{LiveView, MoveView, ScopeView},
};

/// The child content a component invocation passes to its component.
///
/// The children of an invocation reach the component as this view in its
/// props. The component decides where they render by interpolating the
/// value into its own template, which drives the children concurrently
/// with the rest of the template; children that are never interpolated
/// never run.
pub struct Child<'a> {
    view: BoxView<'a>,
}

impl<'a> Child<'a> {
    /// Wraps a view as a component's child content.
    #[must_use]
    pub fn new(view: impl View + 'a) -> Self {
        Self {
            view: Box::pin(view),
        }
    }
}

/// No child content: renders nothing, like a component invoked without
/// children.
impl Default for Child<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

/// An already boxed view becomes child content without boxing again.
impl<'a> From<BoxView<'a>> for Child<'a> {
    fn from(view: BoxView<'a>) -> Self {
        Self { view }
    }
}

/// Implements the conversion from a view type into child content, so a
/// view-typed prop declared as `Child` accepts the type through `#[into]`.
macro_rules! child_from_view {
    ($(#[$doc:meta])* impl<$($param:tt),*> for $ty:ty) => {
        $(#[$doc])*
        impl<'a, $($param),*> From<$ty> for Child<'a>
        where
            $ty: View + 'a,
        {
            fn from(view: $ty) -> Self {
                Self::new(view)
            }
        }
    };
}

child_from_view! {
    /// A `view!` template passes as child content.
    impl<V> for ScopeView<V>
}

child_from_view! {
    /// A `live!` region passes as child content.
    impl<Fut> for LiveView<Fut>
}

child_from_view! {
    /// A captured view block passes as child content.
    impl<Fut> for MoveView<Fut>
}

/// The children's view polls through in place.
impl View for Child<'_> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        self.get_mut().view.as_mut().poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        self.get_mut().view.as_mut().poll_swap(cx)
    }
}
