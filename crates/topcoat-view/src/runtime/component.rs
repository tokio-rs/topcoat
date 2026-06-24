use topcoat_core::runtime::{context::Cx, error::Error};

use crate::runtime::{Props, View};

pub trait Component {
    type Props: Props;

    #[must_use]
    fn props_builder() -> <Self::Props as Props>::Builder {
        Self::Props::builder()
    }

    fn render(
        self,
        cx: &Cx,
        props: Self::Props,
    ) -> impl Future<Output = Result<View, Error>> + Send;
}
