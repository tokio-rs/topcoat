use topcoat_core::{context::Cx, error::Error};

use crate::{Props, View};

pub trait Component {
    type Props: Props;

    #[must_use]
    fn props_builder() -> <Self::Props as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    fn render<'cx>(
        self,
        cx: &'cx Cx,
        props: Self::Props,
    ) -> impl Future<Output = Result<View, Error>> + Send
    where
        Self: 'cx,
        Self::Props: 'cx;
}
