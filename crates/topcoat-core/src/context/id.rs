use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// A unique identifier for the request a [`Cx`] belongs to.
///
/// Each request gets a distinct `CxId`. Every handle derived from the same
/// request, including clones and children made with [`Cx::with`], shares it.
/// Ids are cheap to compare and hash. Read one with [`Cx::id`].
///
/// [`Cx`]: crate::context::Cx
/// [`Cx::id`]: crate::context::Cx::id
/// [`Cx::with`]: crate::context::Cx::with
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

/// The identity of one value registered on a request context.
///
/// A fresh `BindingId` is issued whenever a value is registered on a
/// [`RequestContext`](crate::context::RequestContext), so equal ids always
/// refer to the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindingId(NonZeroU64);

impl BindingId {
    /// Returns a fresh `BindingId` that is distinct from every previously
    /// issued id.
    ///
    /// Ids start at one so that `Option<BindingId>` is the size of a bare id.
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
