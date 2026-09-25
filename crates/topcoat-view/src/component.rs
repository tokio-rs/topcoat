use topcoat_core::{context::Cx, error::Result};

use crate::{Props, View};

pub trait Component {
    /// The component's properties. The lifetime covers data borrowed from
    /// the caller.
    type Props<'a>: Props;

    #[must_use]
    fn props_builder<'a>() -> <Self::Props<'a> as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    ///
    /// The returned view may borrow `cx` and the properties.
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
