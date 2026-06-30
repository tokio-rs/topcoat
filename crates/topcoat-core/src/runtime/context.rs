mod context_map;
mod id;

pub use context_map::*;
pub use id::*;

use std::{any::Any, sync::Arc};

use crate::runtime::{abort::AbortStore, memoize::MemoizeCache};

/// The request context.
///
/// Pages, layouts, components, and routes can take `cx: &Cx` as an optional
/// parameter when they need request-scoped information; Topcoat passes it
/// automatically. Use it to read values registered for the request with
/// [`app_context`] and [`request_context`].
#[derive(Debug)]
pub struct Cx {
    id: CxId,
    app_context: Arc<ContextMap>,
    request_context: ContextMap,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

impl Cx {
    /// Creates a `Cx` from the given app and request context maps.
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>, request_context: ContextMap) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_context,
            memoize_cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
        }
    }

    /// Creates a `Cx` with empty app and request contexts.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::new(Arc::new(ContextMap::new()), ContextMap::new())
    }

    /// Registers `value` on the request context, where it can later be read
    /// back with [`request_context`], returning the value previously registered
    /// for `T`, if any.
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces it and hands back the displaced value.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        self.request_context.insert(value)
    }

    /// Returns `true` if a value of type `T` has been registered on the request
    /// context.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.request_context.contains::<T>()
    }

    /// Returns a reference to the request context value of type `T`, or `None`
    /// if no such value has been registered.
    ///
    /// Equivalent to [`request_context`], but returns an `Option` instead of
    /// panicking when the type is absent.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.request_context.get::<T>()
    }

    /// Returns a mutable reference to the request context value of type `T`, or
    /// `None` if no such value has been registered.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.request_context.get_mut::<T>()
    }

    /// Returns this context's unique [`CxId`].
    #[inline]
    pub fn id(&self) -> CxId {
        self.id
    }
}

impl Default for Cx {
    fn default() -> Self {
        Cx::new(Arc::new(ContextMap::default()), ContextMap::default())
    }
}

#[inline]
#[doc(hidden)]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.memoize_cache
}

#[inline]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.abort_store
}
