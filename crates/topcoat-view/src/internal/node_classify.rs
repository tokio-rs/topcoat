use crate::{
    BoxView, Child, NodeViewParts, View,
    internal::{LiveView, MoveView, ScopeView},
};

/// Splits a node position's value into the parts the template's burst
/// pushes and the view the template's join drives.
///
/// A value implementing [`NodeViewParts`] is all parts: it is pushed into
/// the template's block where the position sits, and the join drives the
/// unit view `()`, which resolves at once to empty content. A view is all
/// unit: nothing is pushed for it, and the join drives it in place and
/// splices the content it resolves at the position.
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
