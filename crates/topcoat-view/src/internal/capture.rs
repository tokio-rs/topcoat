/// Moves a control-flow body's pattern bindings into its nested view.
///
/// A branch or iteration body expands to a [`MoveView`](super::MoveView)
/// whose body borrows its environment, while the values its pattern binds
/// die with the branch or iteration that produced them. The expansion packs
/// those values into this wrapper where they are still alive and takes them
/// back inside the view's body, which then owns them for as long as it
/// lives.
///
/// The wrapper is deliberately not `Copy`, and [`take`](Self::take) consumes
/// it whole: a by-value use of a whole non-`Copy` place is captured by value
/// even in a non-`move` async block. Reading the contents through the field
/// instead would let capture analysis narrow to the possibly `Copy` values
/// inside and downgrade the capture to a borrow, which would not live long
/// enough.
pub struct Capture<T>(pub T);

impl<T> Capture<T> {
    /// Returns the packed bindings, consuming the wrapper.
    pub fn take(self) -> T {
        self.0
    }
}
