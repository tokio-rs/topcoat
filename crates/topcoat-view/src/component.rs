use topcoat_core::{context::Cx, error::Result};

use crate::{Props, View};

pub trait Component {
    /// The component's props, generic over the lifetime of anything they
    /// borrow: a [`Child`](crate::Child), a `&str`, or another reference the
    /// caller hands in.
    ///
    /// The lifetime lives here rather than on the implementing type so that a
    /// component borrowing its props is still a plain unit struct, usable by
    /// its bare name wherever a value is expected.
    type Props<'a>: Props;

    #[must_use]
    fn props_builder<'a>() -> <Self::Props<'a> as Props>::Builder {
        Self::Props::builder()
    }

    /// Renders the component to a [`View`].
    ///
    /// The returned future is the component's body; the [`View`] it resolves
    /// to builds into the buffer of the build it is polled in and may borrow
    /// `cx` and the props.
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
