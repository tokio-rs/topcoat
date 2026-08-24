use crate::{BoxView, View};

/// The child content a component invocation passes to its component.
///
/// The children of an invocation reach the component as this lazy view in
/// its props. The component decides where they render by interpolating the
/// value into its own template, which drives the children concurrently with
/// the rest of the template; children that are never interpolated never
/// run.
pub struct Child<'a> {
    pub(crate) view: BoxView<'a>,
}

impl<'a> Child<'a> {
    /// Wraps a view as a component's child content.
    #[must_use]
    pub fn new(view: impl View + 'a) -> Self {
        Self {
            view: Box::pin(view),
        }
    }
}
