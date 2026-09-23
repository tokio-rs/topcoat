use crate::{
    BoxView, Child, NodeViewParts, View,
    internal::{LiveView, MoveView, ScopeView},
};

/// Splits a node position's value into the parts the template's burst
/// pushes and the view the template's join drives.
///
/// A value implementing [`NodeViewParts`] becomes parts only. It is pushed
/// into the template's block at the position, and the join drives the empty
/// view `()`, which resolves at once. A view becomes a unit only. Nothing is
/// pushed for it, and the join drives it and splices the content it resolves
/// at the position.
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
