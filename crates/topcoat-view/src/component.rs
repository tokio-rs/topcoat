use topcoat_core::{context::Cx, error::Result};

use crate::{Props, View};

pub trait Component {
    type Props: Props;

    #[must_use]
    fn props_builder() -> <Self::Props as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    ///
    /// The returned future is the component's body; the [`View`] it resolves
    /// to may borrow `cx` and the props.
    fn render<'cx>(
        self,
        cx: &'cx Cx,
        props: Self::Props,
    ) -> impl Future<Output = Result<impl View + 'cx>> + Send + 'cx
    where
        Self: 'cx,
        Self::Props: 'cx;
}
