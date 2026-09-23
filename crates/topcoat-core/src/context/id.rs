use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// An identifier for the request shared by a [`Cx`] and its child contexts.
///
/// Cloning a context or adding a scoped value preserves its ID. Retrieve it
/// with [`Cx::id`].
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
