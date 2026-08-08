mod context_map;
mod id;

use std::{any::Any, sync::Arc};

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
    /// While any detached handle is alive, the request context is frozen:
    /// registering a value with [`insert`](Self::insert) panics.
    #[must_use]
    pub fn detach(&self) -> Cx {
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
    /// Panics while a handle taken with [`detach`](Self::detach) is alive.
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
    /// Panics while a handle taken with [`detach`](Self::detach) is alive.
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
    /// Panics while a detached handle is alive, because the state is shared for
    /// as long as the handle exists.
    #[track_caller]
    fn inner_mut(&mut self) -> &mut CxInner {
        Arc::get_mut(&mut self.inner).unwrap_or_else(|| {
            panic!(
                "cannot modify the request context while a handle taken with \
                 `Cx::detach` is alive"
            )
        })
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
    fn inserting_while_a_handle_is_alive_panics() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        let _handle = cx.detach();
        cx.insert(Marker(0));
    }

    #[test]
    fn dropping_every_handle_unfreezes_the_context() {
        let mut cx = Cx::new(Arc::new(ContextMap::new()));
        drop(cx.detach());
        assert_eq!(cx.insert(Marker(0)), None);
    }
}
