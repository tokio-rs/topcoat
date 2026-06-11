use topcoat_core::runtime::{context::Cx, error::Error};

use crate::runtime::{Props, View};

pub trait Component {
    type Props: Props;

    fn render(
        self,
        cx: &Cx,
        props: Self::Props,
    ) -> impl Future<Output = Result<View, Error>> + Send;
}
