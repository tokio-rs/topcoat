/// Counterpart to [`ExprDerefTarget`](super::ExprDerefTarget) for write
/// positions. Server-side `expr_deref_assign` is unreachable — assignment only
/// happens in the browser from inside an event handler.
pub trait ExprDerefAssignTarget {
    type Value;

    fn expr_deref_assign(self, value: Self::Value);
}

/// Assignment through a dereferenced place: `*place = value`. The macro
/// recognises the `*X = Y` pattern and lowers it here rather than wrapping `X`
/// in an [`ExprDeref`](super::ExprDeref) — the latter is a read.
///
/// Type compatibility between place and value is enforced via the
/// [`ExprDerefAssignTarget`] bound, so `*signal = wrong_type` fails to
/// compile.
pub struct ExprAssignDeref<P, V> {
    place: P,
    value: V,
}

impl<P, V> ExprAssignDeref<P, V> {
    pub fn new(place: P, value: V) -> Self {
        Self { place, value }
    }
}
