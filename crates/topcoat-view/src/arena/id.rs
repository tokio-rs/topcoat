use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of an [`Arena`](crate::arena::Arena), unique for the
/// lifetime of the process.
///
/// A view still under construction records the id of the arena its
/// instructions live in, so using it against a different arena fails instead
/// of executing that arena's instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaId(u64);

impl ArenaId {
    pub(crate) fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
