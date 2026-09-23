/// Moves a control-flow body's pattern bindings into its nested view.
///
/// A nested view borrows its environment, but must own pattern bindings
/// that would otherwise be dropped at the end of the branch or iteration.
/// The expansion puts those bindings in this wrapper and moves it into
/// the view's body.
///
/// The wrapper is not `Copy`, and [`take`](Self::take) consumes it. This
/// forces a move even in an async block without `move`. Accessing the field
/// directly could instead borrow a `Copy` value for too short a lifetime.
pub struct Capture<T>(pub T);

impl<T> Capture<T> {
    /// Returns the packed bindings, consuming the wrapper.
    pub fn take(self) -> T {
        self.0
    }
}
