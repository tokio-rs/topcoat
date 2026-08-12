mod context_map;
mod id;

use std::{
    any::Any,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub use context_map::*;
pub use id::*;

pub use crate::memoize::MemoizeAsRef;
use crate::{abort::AbortStore, memoize::MemoizeCache};

/// The request context.
///
/// Pages, layouts, components, and routes can take `cx: &Cx` as an optional
/// parameter when they need request-scoped information; Topcoat passes it
/// automatically. Use it to read values registered for the request with the
/// app and request context helpers, such as [`app_context`] and
/// [`request_context`].
///
/// A `Cx` is a handle to state shared by everything serving the same request.
/// Work that outlives the handler, such as a streaming response body or a
/// WebSocket task, takes an owned handle with [`detach`](Self::detach).
#[derive(Debug, Default)]
pub struct Cx {
    inner: Arc<CxInner>,
}

impl Cx {
    /// Creates the context for one request over the shared app context, with an
    /// empty request context.
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self::from_parts(app_context, ContextMap::new())
    }

    /// Creates a `Cx` from the given app and request context maps.
    fn from_parts(app_context: Arc<ContextMap>, request_context: ContextMap) -> Self {
        Self {
            inner: Arc::new(CxInner {
                id: CxId::new(),
                app_context,
                request_context,
                memoize_cache: MemoizeCache::new(),
                abort_store: AbortStore::new(),
                sealed: AtomicBool::new(false),
            }),
        }
    }

    /// Returns this context's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.inner.id
    }

    /// Returns an owned handle to this request's context.
    ///
    /// Every handle reads the same state: the app context, the request context,
    /// and the memoize cache. Take an owned handle for work that outlives the
    /// handler, such as a streaming response body or a WebSocket task.
    ///
    /// Detaching seals the request context: [`insert`](Self::insert) and
    /// [`get_mut`](Self::get_mut) panic from then on, including after every
    /// detached handle was dropped.
    #[must_use]
    pub fn detach(&self) -> Cx {
        self.inner.sealed.store(true, Ordering::Relaxed);

        Cx {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Registers `value` on the request context, returning the value previously
    /// registered for `T`, if any.
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces it and hands back the displaced value.
    ///
    /// # Panics
    ///
    /// Panics once a handle was taken with [`detach`](Self::detach).
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        self.inner_mut().request_context.insert(value)
    }

    /// Returns a mutable reference to the request context value of type `T`, or
    /// `None` if no such value has been registered.
    ///
    /// # Panics
    ///
    /// Panics once a handle was taken with [`detach`](Self::detach).
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.inner_mut().request_context.get_mut::<T>()
    }

    /// Returns exclusive access to the context state.
    ///
    /// # Panics
    ///
    /// Panics once the context is sealed, because its state is then shared with
    /// handles this one does not know about.
    #[track_caller]
    fn inner_mut(&mut self) -> &mut CxInner {
        assert!(
            !self.inner.sealed.load(Ordering::Relaxed),
            "cannot modify the request context after taking a handle with \
             `Cx::detach`"
        );

        // Only `detach` shares the state, and it seals the context on the way,
        // so an unsealed context is the sole handle to its state.
        Arc::get_mut(&mut self.inner).expect("an unsealed context should be unique")
    }
}

/// The state behind every handle to one request's [`Cx`].
#[derive(Debug, Default)]
struct CxInner {
    id: CxId,
    app_context: Arc<ContextMap>,
    request_context: ContextMap,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
    sealed: AtomicBool,
}

/// Assembles a [`Cx`] from scratch, for tests.
///
/// Unlike [`Cx::new`], which only takes an existing shared app context,
/// `CxTestBuilder` populates both app and request context.
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
        Cx::from_parts(Arc::new(self.app_context), self.request_context)
    }
}

#[inline]
#[must_use]
#[doc(hidden)]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.inner.memoize_cache
}

#[inline]
#[must_use]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.inner.abort_store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Marker(u32);

    #[test]
    fn a_fresh_context_has_a_unique_id() {
        let first = Cx::new(Arc::new(ContextMap::new()));
        let second = Cx::new(Arc::new(ContextMap::new()));
        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn insert_replaces_and_returns_the_displaced_value() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        assert_eq!(cx.insert(Marker(1)), None);
        assert_eq!(cx.insert(Marker(2)), Some(Marker(1)));
        assert_eq!(request_context::<Marker>(&cx), &Marker(2));
    }

    #[test]
    fn get_mut_allows_mutation_in_place() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        assert_eq!(cx.get_mut::<Marker>(), None);
        cx.insert(Marker(1));
        cx.get_mut::<Marker>().unwrap().0 = 42;
        assert_eq!(request_context::<Marker>(&cx), &Marker(42));
    }

    #[test]
    fn detached_handles_outlive_the_original() {
        let cx = CxTestBuilder::new().request_context(Marker(7)).build();
        let id = cx.id();
        let handle = cx.detach();
        drop(cx);

        assert_eq!(request_context::<Marker>(&handle).0, 7);
        assert_eq!(handle.id(), id);
    }

    #[test]
    fn handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Cx>();
    }

    #[test]
    #[should_panic(expected = "`Cx::detach`")]
    fn inserting_after_detaching_panics() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        let _handle = cx.detach();
        cx.insert(Marker(0));
    }

    #[test]
    #[should_panic(expected = "`Cx::detach`")]
    fn mutating_after_detaching_panics() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        cx.insert(Marker(0));
        let _handle = cx.detach();
        let _ = cx.get_mut::<Marker>();
    }

    #[test]
    #[should_panic(expected = "`Cx::detach`")]
    fn dropping_every_handle_keeps_the_context_sealed() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        drop(cx.detach());
        cx.insert(Marker(0));
    }

    #[test]
    #[should_panic(expected = "`Cx::detach`")]
    fn a_detached_handle_cannot_write_to_the_context() {
        let cx = Cx::new(Arc::new(ContextMap::new()));
        let mut handle = cx.detach();
        handle.insert(Marker(0));
    }
}
