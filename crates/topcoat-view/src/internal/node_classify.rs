use crate::{
    BoxView, Child, NodeViewParts, View,
    internal::{LiveView, MoveView, ScopeView},
};

/// Splits a node value into synchronous parts and a view to poll.
///
/// A [`NodeViewParts`] value supplies parts and an empty view. A view
/// supplies no parts and is polled for its content. Both forms preserve
/// the node's position in the template.
pub trait NodeClassify {
    /// The parts the burst pushes at the position.
    type Parts: NodeViewParts;
    /// The view the join drives for the position.
    type Unit: View;

    /// Splits the value into its parts and its view.
    fn classify(self) -> (Self::Parts, Self::Unit);
}

impl<T: NodeViewParts> NodeClassify for T {
    type Parts = T;
    type Unit = ();

    #[inline]
    fn classify(self) -> (Self::Parts, Self::Unit) {
        (self, ())
    }
}

/// Implements [`NodeClassify`] for a view type: the view is the unit, and
/// nothing is pushed for it.
macro_rules! classify_view {
    ($(#[$doc:meta])* impl<$($param:tt),*> for $ty:ty) => {
        $(#[$doc])*
        impl<$($param),*> NodeClassify for $ty
        where
            $ty: View,
        {
            type Parts = ();
            type Unit = Self;

            #[inline]
            fn classify(self) -> (Self::Parts, Self::Unit) {
                ((), self)
            }
        }
    };
}

classify_view! {
    impl<'a> for Child<'a>
}

classify_view! {
    impl<'a> for BoxView<'a>
}

classify_view! {
    impl<Fut> for LiveView<Fut>
}

classify_view! {
    impl<Fut> for MoveView<Fut>
}

classify_view! {
    impl<V> for ScopeView<V>
}
