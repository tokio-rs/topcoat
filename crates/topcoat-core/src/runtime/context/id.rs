use std::sync::atomic::{AtomicU64, Ordering};

/// A unique identifier for a [`Cx`].
///
/// Every [`Cx`] is assigned a distinct `CxId` when it is created, making it
/// cheap to compare and hash. Retrieve a context's id with [`Cx::id`].
///
/// [`Cx`]: crate::runtime::context::Cx
/// [`Cx::id`]: crate::runtime::context::Cx::id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CxId(u64);

impl CxId {
    /// Returns a fresh `CxId` that is distinct from every previously issued ID.
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for CxId {
    fn default() -> Self {
        Self::new()
    }
}
