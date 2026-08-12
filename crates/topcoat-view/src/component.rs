use topcoat_core::{context::Cx, error::Error};

use crate::Props;

pub trait Component {
    type Props: Props;

    #[must_use]
    fn props_builder() -> <Self::Props as Props>::Builder {
        Self::Props::builder()
    }

    /// The component's future: its body runs once, then its render loop
    /// renders once per pass until the request ends or the component is
    /// evicted.
    ///
    /// Takes an owned context handle, so the stored future carries its own
    /// clone of the request state and borrows between component frames carry
    /// only props and locals.
    fn render(self, cx: Cx, props: Self::Props) -> impl Future<Output = Result<(), Error>> + Send;
}
