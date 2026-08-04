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
    sync::Arc,
};

pub use context_map::*;
pub use id::*;
use scoped_tls_hkt::scoped_thread_local;
use smallvec::SmallVec;
pub use test::*;
pub use value::ContextValues;

use crate::{abort::AbortStore, memoize::MemoizeCache};

/// The context for one request.
///
/// Pages, layouts, components, and routes receive `&Cx` when they need
/// request-scoped information. Use [`app_context`] for values shared by every
/// request, [`request_context`] for values in the current request scope, and
/// [`Cx::with`] to create a child scope that temporarily shadows a value.
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
    /// Panics if a scoped context has been leaked with [`std::mem::forget`].
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
        let state = Arc::get_mut(request_state)
            .expect("cannot mutate the request root while a scoped context is still reachable");
        let RequestState {
            root_bindings,
            binding_ids,
            ..
        } = state;
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

    fn request_state_mut(&mut self) -> &mut RequestState {
        Arc::get_mut(&mut self.request_state)
            .expect("cannot mutate the request root while a scoped context is still reachable")
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
