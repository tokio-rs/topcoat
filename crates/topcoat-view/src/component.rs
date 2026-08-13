use topcoat_core::{context::Cx, error::Error};

use crate::{Props, live::Fill};

/// A renderable component: the trait the `#[component]` macro implements for
/// the marker it generates.
pub trait Component {
    /// The component's props for one invoking frame.
    ///
    /// `'frame` is the frame's lifetime: props may borrow the caller's
    /// locals, which live shorter than the request, so the props type is
    /// generic over the frame rather than the request.
    type Props<'frame>: Props
    where
        Self: 'frame;

    #[must_use]
    fn props_builder<'frame>() -> <Self::Props<'frame> as Props>::Builder
    where
        Self: 'frame,
    {
        <Self::Props<'frame> as Props>::builder()
    }

    /// Renders the component as part of a live render.
    ///
    /// The view travels through `fill`, not the return value: the future
    /// delivers its view and keeps running until nothing inside it can
    /// change anymore. Its output is the component's final status; a failure
    /// before the fill surfaces where the component was invoked, and one
    /// after it climbs the frame tree the same way.
    fn render<'frame>(
        self,
        cx: &'frame Cx,
        props: Self::Props<'frame>,
        fill: Fill,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'frame
    where
        Self: 'frame;
}
