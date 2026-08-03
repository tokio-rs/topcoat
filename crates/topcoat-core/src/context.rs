mod binding;
mod context_map;
mod id;
pub mod test;
mod value;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use bit_set::BitSet;
pub use context_map::*;
pub use id::*;
use scoped_tls_hkt::scoped_thread_local;
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
    app_context: Arc<ContextMap>,
    request_state: Arc<RequestState>,
    bindings: binding::BindingSet,
}

impl Cx {
    /// Creates an empty request context over `app_context`.
    #[doc(hidden)]
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_state: Arc::new(RequestState::default()),
            bindings: binding::BindingSet::default(),
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
        self.assert_request_state_unique();
        let mut registry = self.request_state.registry();
        self.bindings.install_root(&mut registry, value)
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
        self.assert_request_state_unique();
        let mut registry = self.request_state.registry();
        self.bindings.get_mut::<T>(&mut registry)
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
        {
            let mut registry = scope.cx.request_state.registry();
            scope.cx.bindings.install_scoped(&mut registry, value);
        }
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
            let mut registry = scope.cx.request_state.registry();
            let mut installer = value::Installer::new(&mut scope.cx.bindings, &mut registry);
            <V as value::Sealed>::install(values, &mut installer);
        }
        scope
    }

    fn child(&self) -> CxScope<'_> {
        CxScope {
            cx: Self {
                id: CxId::new(),
                app_context: self.app_context.clone(),
                request_state: self.request_state.clone(),
                bindings: self.bindings.clone(),
            },
            parent: PhantomData,
        }
    }

    fn assert_request_state_unique(&mut self) {
        Arc::get_mut(&mut self.request_state)
            .expect("cannot mutate the request root while a scoped context is still reachable");
    }

    fn app_context_mut(&mut self) -> &mut ContextMap {
        Arc::get_mut(&mut self.app_context).expect("test context app context is still shared")
    }

    pub(crate) fn request_value<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        if let Some(binding) = self.bindings.get::<T>() {
            record_context_read(self, binding.id);
            return Some(&binding.value);
        }

        let mut registry = self.request_state.registry();
        let binding_id = registry.root_none(TypeId::of::<T>());
        record_context_read_with_registry(self, binding_id, &registry);
        None
    }

    pub(crate) fn context_reads_match(&self, reads: &ContextReadMask) -> bool {
        let binding_mask = &self.bindings.mask;
        if reads.frontier <= binding_mask.frontier {
            reads.bits.is_subset(&binding_mask.bits)
        } else {
            let registry = self.request_state.registry();
            binding_mask.contains_reads(&reads.bits, &registry)
        }
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
    registry: Mutex<binding::Registry>,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

impl RequestState {
    fn registry(&self) -> std::sync::MutexGuard<'_, binding::Registry> {
        self.registry.lock().unwrap()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ContextReadMask {
    bits: BitSet<usize>,
    frontier: usize,
}

impl ContextReadMask {
    fn insert(&mut self, binding_id: binding::Id) {
        self.bits.insert(binding_id.0);
        self.frontier = self.frontier.max(binding_id.frontier());
    }

    fn union_with(&mut self, reads: &Self) {
        self.bits.union_with(&reads.bits);
        self.frontier = self.frontier.max(reads.frontier);
    }

    fn union_visible(
        &mut self,
        reads: &Self,
        binding_mask: &binding::Mask,
        registry: &binding::Registry,
    ) {
        binding_mask.union_visible_reads(&mut self.bits, &reads.bits, registry);
        self.frontier = self.frontier.max(reads.frontier);
    }
}

pub(crate) struct ContextTracker<'cx> {
    cx: &'cx Cx,
    reads: RefCell<ContextReadMask>,
}

impl<'cx> ContextTracker<'cx> {
    pub(crate) fn new(cx: &'cx Cx) -> Self {
        Self {
            cx,
            reads: RefCell::new(ContextReadMask::default()),
        }
    }

    pub(crate) fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
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

    pub(crate) fn finish(&self) -> ContextReadMask {
        self.reads.take()
    }

    fn record(&self, binding_id: binding::Id) {
        if binding_id.0 < self.cx.bindings.mask.frontier {
            if self.cx.bindings.mask.bits.contains(binding_id.0) {
                self.reads.borrow_mut().insert(binding_id);
            }
            return;
        }

        let registry = self.cx.request_state.registry();
        self.record_with_registry(binding_id, &registry);
    }

    fn record_with_registry(&self, binding_id: binding::Id, registry: &binding::Registry) {
        if !self
            .cx
            .bindings
            .mask
            .effectively_contains(registry, binding_id)
        {
            return;
        }
        self.reads.borrow_mut().insert(binding_id);
    }

    fn record_reads(&self, cx: &Cx, reads: &ContextReadMask) {
        if !Arc::ptr_eq(&self.cx.request_state, &cx.request_state) {
            return;
        }

        let mut tracked = self.reads.borrow_mut();
        if std::ptr::eq(self.cx, cx) {
            tracked.union_with(reads);
        } else {
            let registry = self.cx.request_state.registry();
            tracked.union_visible(reads, &self.cx.bindings.mask, &registry);
        }
    }

    fn merge_into(&self, tracker: &ContextTracker<'_>) {
        let reads = self.reads.borrow();
        tracker.record_reads(self.cx, &reads);
    }
}

scoped_thread_local!(static ACTIVE_TRACKER: for<'a> &'a ContextTracker<'a>);

struct TrackerScope<'tracker, 'tracker_cx, 'previous, 'previous_cx> {
    tracker: &'tracker ContextTracker<'tracker_cx>,
    previous: &'previous ContextTracker<'previous_cx>,
}

impl Drop for TrackerScope<'_, '_, '_, '_> {
    fn drop(&mut self) {
        self.tracker.merge_into(self.previous);
    }
}

fn record_context_read(cx: &Cx, binding_id: binding::Id) {
    if ACTIVE_TRACKER.is_set() {
        ACTIVE_TRACKER.with(|tracker| {
            if Arc::ptr_eq(&tracker.cx.request_state, &cx.request_state) {
                tracker.record(binding_id);
            }
        });
    }
}

fn record_context_read_with_registry(
    cx: &Cx,
    binding_id: binding::Id,
    registry: &binding::Registry,
) {
    if ACTIVE_TRACKER.is_set() {
        ACTIVE_TRACKER.with(|tracker| {
            if Arc::ptr_eq(&tracker.cx.request_state, &cx.request_state) {
                tracker.record_with_registry(binding_id, registry);
            }
        });
    }
}

pub(crate) fn replay_context_reads(cx: &Cx, reads: &ContextReadMask) {
    if ACTIVE_TRACKER.is_set() {
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
