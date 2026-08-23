use futures_core::Stream;
use topcoat_core::{context::Cx, error::Error};

use crate::{Props, ViewChunk};

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
    ) -> impl Stream<Item = Result<ViewChunk, Error>> + Send
    where
        Self: 'cx,
        Self::Props: 'cx;
}
