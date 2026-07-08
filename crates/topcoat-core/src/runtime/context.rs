mod context_map;
mod id;

pub use context_map::*;
pub use id::*;

use std::{any::Any, ops::Deref, sync::Arc};

use crate::runtime::{abort::AbortStore, memoize::MemoizeCache};

/// The request context.
///
/// Pages, layouts, components, and routes can take `cx: &Cx` as an optional
/// parameter when they need request-scoped information; Topcoat passes it
/// automatically. Use it to read values registered for the request with
/// [`app_context`] and [`request_context`].
#[derive(Debug, Default)]
pub struct Cx {
    id: CxId,
    app_context: Arc<ContextMap>,
    request_context: ContextMap,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

impl Cx {
    /// Creates a `Cx` from the given app and request context maps.
    fn new(app_context: Arc<ContextMap>, request_context: ContextMap) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_context,
            memoize_cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
        }
    }

    /// Returns this context's unique [`CxId`].
    #[inline]
    pub fn id(&self) -> CxId {
        self.id
    }
}

/// Assembles the request context for an in-flight request.
///
/// The router creates a `CxBuilder` over the shared app context, then threads
/// `&mut CxBuilder` through the layers wrapping the matched route so each can
/// register request-scoped values with [`insert`](Self::insert) before the
/// route runs. Because a `CxBuilder` dereferences to the [`Cx`] it is building,
/// app and request context can be read through it (with [`app_context`] and
/// [`request_context`]) while it is still being assembled.
#[derive(Debug, Default)]
pub struct CxBuilder {
    cx: Cx,
}

impl CxBuilder {
    /// Creates a builder over the shared app context, with an empty request
    /// context.
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self {
            cx: Cx::new(app_context, ContextMap::new()),
        }
    }

    /// Registers `value` on the request context, returning the value previously
    /// registered for `T`, if any.
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces it and hands back the displaced value.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        self.cx.request_context.insert(value)
    }

    /// Returns `true` if a value of type `T` has been registered on the request
    /// context.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.cx.request_context.contains::<T>()
    }

    /// Returns a reference to the request context value of type `T`, or `None`
    /// if no such value has been registered.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.cx.request_context.get::<T>()
    }

    /// Returns a mutable reference to the request context value of type `T`, or
    /// `None` if no such value has been registered.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.cx.request_context.get_mut::<T>()
    }

    /// Consumes the builder, returning the finished [`Cx`].
    #[must_use]
    pub fn build(self) -> Cx {
        self.cx
    }
}

impl Deref for CxBuilder {
    type Target = Cx;

    fn deref(&self) -> &Cx {
        &self.cx
    }
}

/// Assembles a [`Cx`] from scratch, for tests.
///
/// Unlike [`CxBuilder`], which only configures request context over an existing
/// shared app context, `CxTestBuilder` populates both app and request context.
#[derive(Debug, Default)]
pub struct CxTestBuilder {
    app_context: ContextMap,
    request_context: ContextMap,
}

impl CxTestBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` on the app context.
    #[must_use]
    pub fn app_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.app_context.insert(value);
        self
    }

    /// Registers `value` on the request context.
    #[must_use]
    pub fn request_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.request_context.insert(value);
        self
    }

    /// Consumes the builder, returning the assembled [`Cx`].
    #[must_use]
    pub fn build(self) -> Cx {
        Cx::new(Arc::new(self.app_context), self.request_context)
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
