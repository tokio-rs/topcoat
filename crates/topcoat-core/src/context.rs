mod binding;
mod context_map;
mod id;
mod test;
mod value;

use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    marker::PhantomData,
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub use context_map::*;
pub use id::*;
use scoped_tls_hkt::scoped_thread_local;
use smallvec::SmallVec;
pub use test::*;
pub use value::ContextValues;

pub use crate::memoize::MemoizeAsRef;
use crate::{abort::AbortStore, memoize::MemoizeCache};

/// The context for one request.
///
/// Pages, layouts, components, and routes receive `&Cx` when they need
/// request-scoped information. Use [`app_context`] for values shared by every
/// request, [`request_context`] for values in the current request scope, and
/// [`Cx::with`] to create a child scope that temporarily shadows a value.
///
/// A `Cx` is a handle to state shared by everything serving the same request.
/// Work that outlives the handler, such as a streaming response body or a
/// WebSocket task, takes an owned handle with [`detach`](Self::detach).
pub struct Cx {
    id: CxId,
    cache_id: CacheId,
    app_context: Arc<ContextMap>,
    request_state: Arc<RequestState>,
    scoped_bindings: Option<binding::ScopedBindings>,
}

impl Cx {
    /// Creates an empty request context over `app_context`.
    #[doc(hidden)]
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self {
            id: CxId::new(),
            cache_id: CacheId::ROOT,
            app_context,
            request_state: Arc::new(RequestState::default()),
            scoped_bindings: None,
        }
    }

    /// Returns this context's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.id
    }

    /// Returns an owned handle to this request's context.
    ///
    /// Every handle reads the same state: the app context, the request
    /// bindings visible from this context, and the memoize cache. Take an
    /// owned handle for work that outlives the handler, such as a streaming
    /// response body or a WebSocket task.
    ///
    /// Detaching seals the request root: [`insert`](Self::insert) and
    /// [`get_mut`](Self::get_mut) panic from then on, including after every
    /// detached handle was dropped.
    #[must_use]
    pub fn detach(&self) -> Cx {
        self.request_state.sealed.store(true, Ordering::Relaxed);

        Self {
            id: self.id,
            cache_id: self.cache_id,
            app_context: self.app_context.clone(),
            request_state: self.request_state.clone(),
            scoped_bindings: self.scoped_bindings.clone(),
        }
    }

    /// Registers `value` at the request root, returning the displaced value.
    ///
    /// A type has one root binding at a time. Replacing it gives the new
    /// binding a fresh identity, so memoized functions that read the previous
    /// binding will recompute when called through the root or a scope that
    /// inherits it.
    ///
    /// A scope or reference borrowed from the context must end before root
    /// mutation:
    ///
    /// ```compile_fail
    /// use topcoat::context::Cx;
    ///
    /// let mut cx = Cx::default();
    /// let _scope = cx.with(1u8);
    /// cx.insert(2u8);
    /// ```
    ///
    /// ```compile_fail
    /// use topcoat::context::{CxTestBuilder, request_context};
    ///
    /// let mut cx = CxTestBuilder::new().request_context(1u8).build();
    /// let value = request_context::<u8>(&cx);
    /// cx.insert(2u8);
    /// dbg!(value);
    /// ```
    ///
    /// Memoized results and futures borrow the context in the same way:
    ///
    /// ```compile_fail
    /// use topcoat::context::{Cx, memoize};
    ///
    /// #[memoize]
    /// fn value(cx: &Cx) -> u8 { 1 }
    ///
    /// let mut cx = Cx::default();
    /// let cached = value(&cx);
    /// cx.insert(2u8);
    /// dbg!(cached);
    /// ```
    ///
    /// ```compile_fail
    /// use topcoat::context::{Cx, memoize};
    ///
    /// #[memoize]
    /// async fn value(cx: &Cx) -> u8 { 1 }
    ///
    /// async fn example() {
    ///     let mut cx = Cx::default();
    ///     let pending = value(&cx);
    ///     cx.insert(2u8);
    ///     pending.await;
    /// }
    /// ```
    ///
    /// This method is intended for the root context. Calling it after leaking
    /// a scoped context is an invariant violation and panics.
    ///
    /// # Panics
    ///
    /// Panics once a handle was taken with [`detach`](Self::detach).
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        let (previous, id) = {
            let state = self.request_state_mut();
            state.root_bindings.install(&state.binding_ids, value)
        };
        self.cache_id = CacheId::from(id);
        previous
    }

    /// Returns mutable access to a request-root value.
    ///
    /// A successful lookup gives the binding a fresh identity before returning
    /// the reference. This invalidates memoized results that observed the old
    /// identity even when the value is left unchanged. A missing lookup does
    /// not allocate an identity or invalidate anything.
    ///
    /// # Panics
    ///
    /// Panics once a handle was taken with [`detach`](Self::detach), or if a
    /// scoped context has been leaked with [`std::mem::forget`].
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        let Self {
            cache_id,
            request_state,
            ..
        } = self;
        let RequestState {
            root_bindings,
            binding_ids,
            ..
        } = RequestState::exclusive(request_state);
        let (value, binding_id) = root_bindings.get_mut::<T>()?;
        let id = binding_ids.allocate();
        *binding_id = id;
        *cache_id = CacheId::from(id);
        Some(value)
    }

    /// Creates a child scope containing `value`.
    ///
    /// The returned context resolves `value` before bindings of the same type
    /// in this context. Other request values remain inherited. Use
    /// [`with_values`](Self::with_values) to add several separate bindings.
    pub fn with<T>(&self, value: T) -> CxScope<'_>
    where
        T: Any + Send + Sync,
    {
        let mut scope = self.child();
        let id = scope
            .cx
            .scoped_bindings
            .get_or_insert_default()
            .install(&scope.cx.request_state.binding_ids, value);
        scope.cx.cache_id = CacheId::from(id);
        scope
    }

    /// Creates a child scope containing each tuple element as a separate
    /// typed binding.
    ///
    /// Use [`with`](Self::with) when the tuple itself should be one binding.
    ///
    /// # Panics
    ///
    /// Panics if `values` contains the same type more than once.
    pub fn with_values<V>(&self, values: V) -> CxScope<'_>
    where
        V: ContextValues,
    {
        <V as value::Sealed>::assert_unique();
        let mut scope = self.child();
        {
            let mut installer = value::Installer::new(
                scope.cx.scoped_bindings.get_or_insert_default(),
                &scope.cx.request_state.binding_ids,
                &mut scope.cx.cache_id,
            );
            <V as value::Sealed>::install(values, &mut installer);
        }
        scope
    }

    fn child(&self) -> CxScope<'_> {
        CxScope {
            cx: Self {
                id: CxId::new(),
                cache_id: self.cache_id,
                app_context: self.app_context.clone(),
                request_state: self.request_state.clone(),
                scoped_bindings: self.scoped_bindings.clone(),
            },
            parent: PhantomData,
        }
    }

    #[track_caller]
    fn request_state_mut(&mut self) -> &mut RequestState {
        RequestState::exclusive(&mut self.request_state)
    }

    fn app_context_mut(&mut self) -> &mut ContextMap {
        Arc::get_mut(&mut self.app_context).expect("test context app context is still shared")
    }

    #[inline]
    pub(crate) fn request_value<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        if let Some(binding) = self
            .scoped_bindings
            .as_ref()
            .and_then(binding::ScopedBindings::get::<T>)
        {
            track_context_read::<T>(self, Some(binding.id));
            return Some(binding.value());
        }

        let binding = self.request_state.root_bindings.get::<T>();
        track_context_read::<T>(self, binding.map(|binding| binding.id));
        binding.map(|binding| &binding.value)
    }

    pub(crate) fn context_reads_match(&self, reads: &[ContextRead]) -> bool {
        reads.iter().all(|read| read.matches(self))
    }

    #[inline]
    pub(crate) fn cache_id(&self) -> CacheId {
        self.cache_id
    }

    fn binding_id<T>(&self) -> Option<binding::Id>
    where
        T: Any + Send + Sync,
    {
        if let Some(binding) = self
            .scoped_bindings
            .as_ref()
            .and_then(binding::ScopedBindings::get::<T>)
        {
            return Some(binding.id);
        }

        self.request_state
            .root_bindings
            .get::<T>()
            .map(|binding| binding.id)
    }
}

impl Default for Cx {
    fn default() -> Self {
        Self::new(Arc::new(ContextMap::new()))
    }
}

impl std::fmt::Debug for Cx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cx")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// A child request context created by [`Cx::with`] or [`Cx::with_values`].
///
/// It owns its bindings, dereferences to [`Cx`], and cannot outlive the context
/// that created it. It does not provide mutable access to the root context.
#[must_use = "a scoped context has no effect unless it is passed to other work"]
pub struct CxScope<'cx> {
    cx: Cx,
    parent: PhantomData<&'cx Cx>,
}

impl Deref for CxScope<'_> {
    type Target = Cx;

    fn deref(&self) -> &Self::Target {
        &self.cx
    }
}

// Keep the parent borrow live until the scope's shared state and binding map
// are dropped. This makes a mutable root borrow sufficient proof that its
// request state and root bindings are uniquely owned.
impl Drop for CxScope<'_> {
    fn drop(&mut self) {}
}

impl std::fmt::Debug for CxScope<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cx.fmt(f)
    }
}

#[derive(Debug, Default)]
struct RequestState {
    root_bindings: binding::RootBindings,
    binding_ids: binding::IdAllocator,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
    sealed: AtomicBool,
}

impl RequestState {
    /// Returns exclusive access to the request state behind `state`.
    ///
    /// # Panics
    ///
    /// Panics once the request was sealed with [`Cx::detach`], or while a
    /// leaked scoped context still shares the state.
    #[track_caller]
    fn exclusive(state: &mut Arc<Self>) -> &mut Self {
        assert!(
            !state.sealed.load(Ordering::Relaxed),
            "cannot modify the request context after taking a handle with \
             `Cx::detach`"
        );

        Arc::get_mut(state)
            .expect("cannot mutate the request root while a scoped context is still reachable")
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ContextRead {
    type_id: TypeId,
    binding_id: Option<binding::Id>,
    resolve: fn(&Cx) -> Option<binding::Id>,
}

impl ContextRead {
    fn new<T>(binding_id: Option<binding::Id>) -> Self
    where
        T: Any + Send + Sync,
    {
        Self {
            type_id: TypeId::of::<T>(),
            binding_id,
            resolve: Cx::binding_id::<T>,
        }
    }

    fn matches(self, cx: &Cx) -> bool {
        (self.resolve)(cx) == self.binding_id
    }
}

impl PartialEq for ContextRead {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.binding_id == other.binding_id
    }
}

impl Eq for ContextRead {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheId(usize);

impl CacheId {
    const ROOT: Self = Self(usize::MAX);

    #[inline]
    pub(crate) fn get(self) -> usize {
        self.0
    }
}

impl From<binding::Id> for CacheId {
    fn from(id: binding::Id) -> Self {
        Self(id.get())
    }
}

pub(crate) struct ContextTracker<'cx> {
    cx: &'cx Cx,
    reads: RefCell<SmallVec<[ContextRead; 4]>>,
}

impl<'cx> ContextTracker<'cx> {
    pub(crate) fn new(cx: &'cx Cx) -> Self {
        Self {
            cx,
            reads: RefCell::new(SmallVec::new()),
        }
    }

    pub(crate) fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
        let _tracking = TrackingScope::enter();
        if ACTIVE_TRACKER.is_set() {
            ACTIVE_TRACKER.with(|previous| {
                let _scope = TrackerScope {
                    tracker: self,
                    previous,
                };
                ACTIVE_TRACKER.set(self, f)
            })
        } else {
            ACTIVE_TRACKER.set(self, f)
        }
    }

    pub(crate) fn finish(&self) -> SmallVec<[ContextRead; 4]> {
        self.reads.take()
    }

    fn record(&self, cx: &Cx, read: ContextRead) {
        if !std::ptr::eq(self.cx, cx) && !read.matches(self.cx) {
            return;
        }

        let mut reads = self.reads.borrow_mut();
        if let Some(previous) = reads
            .iter()
            .find(|previous| previous.type_id == read.type_id)
        {
            debug_assert_eq!(*previous, read);
        } else {
            reads.push(read);
        }
    }

    fn record_reads(&self, cx: &Cx, reads: &[ContextRead]) {
        if !Arc::ptr_eq(&self.cx.request_state, &cx.request_state) {
            return;
        }

        for &read in reads {
            self.record(cx, read);
        }
    }

    fn merge_into(&self, tracker: &ContextTracker<'_>) {
        let reads = self.reads.borrow();
        tracker.record_reads(self.cx, &reads);
    }
}

scoped_thread_local!(static ACTIVE_TRACKER: for<'a> &'a ContextTracker<'a>);

thread_local! {
    static TRACKING_CONTEXT: Cell<bool> = const { Cell::new(false) };
}

struct TrackingScope {
    previous: bool,
}

impl TrackingScope {
    fn enter() -> Self {
        Self {
            previous: TRACKING_CONTEXT.replace(true),
        }
    }
}

impl Drop for TrackingScope {
    fn drop(&mut self) {
        TRACKING_CONTEXT.set(self.previous);
    }
}

struct TrackerScope<'tracker, 'tracker_cx, 'previous, 'previous_cx> {
    tracker: &'tracker ContextTracker<'tracker_cx>,
    previous: &'previous ContextTracker<'previous_cx>,
}

impl Drop for TrackerScope<'_, '_, '_, '_> {
    fn drop(&mut self) {
        self.tracker.merge_into(self.previous);
    }
}

#[inline]
fn track_context_read<T>(cx: &Cx, binding_id: Option<binding::Id>)
where
    T: Any + Send + Sync,
{
    if TRACKING_CONTEXT.get() {
        let read = ContextRead::new::<T>(binding_id);
        ACTIVE_TRACKER.with(|tracker| {
            if Arc::ptr_eq(&tracker.cx.request_state, &cx.request_state) {
                tracker.record(cx, read);
            }
        });
    }
}

pub(crate) fn replay_context_reads(cx: &Cx, reads: &[ContextRead]) {
    if TRACKING_CONTEXT.get() {
        ACTIVE_TRACKER.with(|tracker| tracker.record_reads(cx, reads));
    }
}

#[inline]
#[doc(hidden)]
#[must_use]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.request_state.memoize_cache
}

#[inline]
#[doc(hidden)]
#[must_use]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.request_state.abort_store
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
    fn a_detached_handle_keeps_its_scoped_bindings() {
        let cx = CxTestBuilder::new().request_context(Marker(1)).build();
        let scope = cx.with(Marker(2));
        let handle = scope.detach();
        drop(scope);
        drop(cx);

        assert_eq!(request_context::<Marker>(&handle).0, 2);
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
