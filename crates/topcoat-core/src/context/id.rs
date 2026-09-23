use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// A unique identifier for a [`Cx`].
///
/// Every [`Cx`] is assigned a distinct `CxId` when it is created, making it
/// cheap to compare and hash. Retrieve a context's id with [`Cx::id`].
///
/// [`Cx`]: crate::context::Cx
/// [`Cx::id`]: crate::context::Cx::id
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

/// The identity of one request context binding.
///
/// A fresh `BindingId` is issued whenever a value is registered on a request
/// context, so equal ids always refer to the same value.
///
/// Ids start at one so that `Option<BindingId>`, the shape a recorded read
/// stores, is the size of a bare id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindingId(NonZeroU64);

impl BindingId {
    /// Returns a fresh `BindingId` that is distinct from every previously
    /// issued id.
    ///
    /// # Panics
    ///
    /// Panics once more than `u64::MAX` ids were issued for this process.
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroU64::new(id).expect("binding id counter wrapped around"))
    }
}
