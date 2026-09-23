use topcoat_core::{context::Cx, error::Result};

use crate::{Props, View};

/// A reusable view with typed properties.
pub trait Component {
    /// The component's properties, which may borrow data for `'a`.
    type Props<'a>: Props;

    /// Returns a builder for the component's properties.
    #[must_use]
    fn props_builder<'a>() -> <Self::Props<'a> as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    ///
    /// The returned view may borrow the context and properties.
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
