use topcoat_core::{context::Cx, error::Result};

use crate::{Props, View};

/// A reusable piece of a template that takes props and renders a [`View`].
///
/// Components are defined with the
/// [`#[component]`](https://docs.rs/topcoat/latest/topcoat/view/attr.component.html)
/// attribute, which turns an async function into a unit struct implementing
/// this trait. A `view!` template invokes a component by name, builds its
/// props, and renders it in place. Implementing the trait by hand is rarely
/// needed.
pub trait Component {
    /// The component's props, generic over the lifetime of anything they
    /// borrow, such as a [`Child`](crate::Child) or a `&str`.
    ///
    /// The lifetime is on the props rather than on the implementing type, so
    /// a component that borrows its props is still a plain unit struct that
    /// can be named as a value.
    type Props<'a>: Props;

    /// Returns a builder for the component's props with no properties set.
    #[must_use]
    fn props_builder<'a>() -> <Self::Props<'a> as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    ///
    /// The returned future is the component's body. The [`View`] it resolves
    /// to may borrow `cx` and the props.
    fn render<'cx, 'a>(
        self,
        cx: &'cx Cx,
        props: Self::Props<'a>,
    ) -> impl Future<Output = Result<impl View + 'cx>> + Send + 'cx
    where
        'a: 'cx,
        Self: 'cx,
        Self::Props<'a>: 'cx;
}
