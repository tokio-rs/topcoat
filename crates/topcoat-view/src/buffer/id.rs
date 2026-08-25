use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of a [`ViewBuffer`](crate::buffer::ViewBuffer), unique for
/// the lifetime of the process.
///
/// A view still under construction records the id of the buffer its
/// instructions live in, so using it against a different buffer fails instead
/// of executing that buffer's instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewBufferId(u64);

impl ViewBufferId {
    pub(super) fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
