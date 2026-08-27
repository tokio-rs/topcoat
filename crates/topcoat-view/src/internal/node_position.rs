use std::pin::Pin;

use super::{Builder, LiveView, MoveView, Pending, ScopeView, Slotted, ThenView};
use crate::{BoxView, Child, NodeViewParts, View};

/// A value filling a node position of a template.
///
/// The position's type decides how it renders. A value implementing
/// [`NodeViewParts`] is pushed into the block right where the position
/// sits, and nothing is left to drive. A view reserves the position and is
/// driven after the block was built, filling the position with the content
/// it resolves.
pub trait NodePosition {
    /// What is left to drive after the position was rendered.
    type Pending: Pending;

    /// Renders the position into the block and returns what is left to
    /// drive.
    fn render(self, b: &mut Builder<'_>) -> Self::Pending;
}

/// A value is pushed into the block at once.
impl<T: NodeViewParts> NodePosition for T {
    type Pending = ();

    #[inline]
    fn render(self, b: &mut Builder<'_>) {
        b.node(self);
    }
}

/// Implements [`NodePosition`] for a view type, which reserves the position
/// and is driven after the block was built.
///
/// A view that is `Unpin` is driven in place; any other is pinned on the
/// heap when it becomes a pending.
macro_rules! position_view {
    ($(#[$doc:meta])* impl<$($param:tt),*> for $ty:ty) => {
        $(#[$doc])*
        impl<$($param),*> NodePosition for $ty
        where
            $ty: View + Unpin,
        {
            type Pending = Slotted<Self>;

            #[inline]
            fn render(self, b: &mut Builder<'_>) -> Self::Pending {
                Slotted::new(self, b.parts().reserve())
            }
        }
    };
    ($(#[$doc:meta])* impl<$($param:tt),*> for $ty:ty => boxed) => {
        $(#[$doc])*
        impl<$($param),*> NodePosition for $ty
        where
            $ty: View,
        {
            type Pending = Slotted<Pin<Box<Self>>>;

            #[inline]
            fn render(self, b: &mut Builder<'_>) -> Self::Pending {
                Slotted::new(Box::pin(self), b.parts().reserve())
            }
        }
    };
}

position_view! {
    /// A component's child content.
    impl<'a> for Child<'a>
}

position_view! {
    /// A boxed view.
    impl<'a> for BoxView<'a>
}

position_view! {
    /// A `live!` region.
    impl<Fut> for LiveView<Fut> => boxed
}

position_view! {
    /// A nested `view!` invocation's template.
    impl<Fut> for MoveView<Fut> => boxed
}

position_view! {
    /// A nested `view!` invocation.
    impl<V> for ScopeView<V> => boxed
}

position_view! {
    /// A component invocation.
    impl<F, V> for ThenView<F, V> => boxed
}
