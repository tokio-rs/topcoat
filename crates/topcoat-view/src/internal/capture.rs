/// Moves the bindings of the control-flow patterns enclosing a component
/// invocation into its child content.
///
/// The children of an invocation expand to a [`MoveView`](super::MoveView)
/// whose body borrows its environment, while the values the enclosing
/// patterns bind die with the branch or iteration that produced them. The
/// expansion packs the values the children mention into this wrapper where
/// they are still alive and takes them back inside the view's body, which
/// then owns them for as long as it lives.
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
