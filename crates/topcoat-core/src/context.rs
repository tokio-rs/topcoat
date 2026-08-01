mod context_map;
mod id;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

pub use context_map::*;
pub use id::*;

use crate::{abort::AbortStore, memoize::MemoizeCache};

type ContextValue = Box<dyn Any + Send + Sync>;
type ContextSnapshot = HashMap<TypeId, ContextBindingId>;
type BindingMap = im::HashMap<TypeId, Arc<ContextBinding>>;

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
    scoped_bindings: Option<BindingMap>,
}

impl Cx {
    /// Creates an empty request context over `app_context`.
    #[doc(hidden)]
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self::from_values(app_context, Vec::new())
    }

    fn from_values(app_context: Arc<ContextMap>, values: Vec<ContextValue>) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_state: Arc::new(RequestState::new(values)),
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
        let state = self.request_state_mut();
        let binding_id = state.next_binding_id();
        let binding = ContextBinding::new(binding_id, Box::new(value));
        state
            .root
            .insert(TypeId::of::<T>(), Arc::new(binding))
            .map(ContextBinding::into_value)
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
        let state = self.request_state_mut();
        let type_id = TypeId::of::<T>();
        if !state.root.contains_key(&type_id) {
            return None;
        }

        let binding_id = state.next_binding_id();
        let binding = state
            .root
            .get_mut(&type_id)
            .expect("context binding disappeared");
        let binding = Arc::get_mut(binding)
            .expect("request root binding is still shared with a scoped context");
        binding.id = binding_id;
        binding.value.downcast_mut::<T>()
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
        self.with_erased_values(vec![Box::new(value)])
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
        let values = values.into_context_values();
        let mut types = HashSet::with_capacity(values.len());
        assert!(
            values
                .iter()
                .all(|value| types.insert(value.as_ref().type_id())),
            "a context scope cannot contain duplicate value types"
        );
        self.with_erased_values(values)
    }

    fn with_erased_values(&self, values: Vec<ContextValue>) -> CxScope<'_> {
        let mut bindings = self.visible_binding_map().clone();
        for value in values {
            let type_id = value.as_ref().type_id();
            let binding_id = self.request_state.next_binding_id();
            bindings.insert(type_id, Arc::new(ContextBinding::new(binding_id, value)));
        }
        CxScope {
            cx: Cx {
                id: CxId::new(),
                app_context: self.app_context.clone(),
                request_state: self.request_state.clone(),
                scoped_bindings: Some(bindings),
            },
            parent: PhantomData,
        }
    }

    fn request_state_mut(&mut self) -> &mut RequestState {
        Arc::get_mut(&mut self.request_state)
            .expect("cannot mutate the request root while a scoped context is still reachable")
    }

    fn visible_binding_map(&self) -> &BindingMap {
        self.scoped_bindings
            .as_ref()
            .unwrap_or(&self.request_state.root)
    }

    pub(crate) fn request_value<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let resolved = self.resolve_value::<T>();
        record_context_read(
            &self.request_state,
            ContextRead {
                type_id,
                binding_id: resolved.map(|(_, binding_id)| binding_id),
            },
        );
        resolved.map(|(value, _)| value)
    }

    fn resolve_value<T>(&self) -> Option<(&T, ContextBindingId)>
    where
        T: Any + Send + Sync,
    {
        self.visible_binding_map()
            .get(&TypeId::of::<T>())
            .map(|binding| {
                (
                    binding
                        .value
                        .downcast_ref::<T>()
                        .expect("context binding type changed"),
                    binding.id,
                )
            })
    }

    pub(crate) fn resolve_binding_id(&self, type_id: TypeId) -> Option<ContextBindingId> {
        self.visible_binding_map()
            .get(&type_id)
            .map(|binding| binding.id)
    }

    pub(crate) fn visible_bindings(&self) -> ContextSnapshot {
        self.visible_binding_map()
            .iter()
            .map(|(&type_id, binding)| (type_id, binding.id))
            .collect()
    }

    pub(crate) fn context_reads_match(&self, reads: &[ContextRead]) -> bool {
        reads
            .iter()
            .all(|read| self.resolve_binding_id(read.type_id) == read.binding_id)
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

/// A tuple of values that can be installed by [`Cx::with_values`].
///
/// This trait is implemented for tuples containing two through twelve values.
/// It is sealed and cannot be implemented outside Topcoat.
pub trait ContextValues: private::Sealed {
    #[doc(hidden)]
    fn into_context_values(self) -> Vec<ContextValue>;
}

macro_rules! impl_context_values {
    (@one $(($type:ident, $index:tt)),+) => {
        impl<$($type),+> private::Sealed for ($($type,)+)
        where
            $($type: Any + Send + Sync,)+
        {}

        impl<$($type),+> ContextValues for ($($type,)+)
        where
            $($type: Any + Send + Sync,)+
        {
            fn into_context_values(self) -> Vec<ContextValue> {
                vec![$(Box::new(self.$index),)+]
            }
        }
    };
    (@each [$(($type:ident, $index:tt)),+] ($next:ident, $next_index:tt) $(, $rest:tt)*) => {
        impl_context_values!(@one $(($type, $index)),+);
        impl_context_values!(@each [$(($type, $index)),+, ($next, $next_index)] $($rest),*);
    };
    (@each [$(($type:ident, $index:tt)),+]) => {
        impl_context_values!(@one $(($type, $index)),+);
    };
}

impl_context_values!(@each [(T1, 0), (T2, 1)]
    (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6),
    (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11)
);

mod private {
    pub trait Sealed {}
}

/// Assembles a [`Cx`] from scratch for tests.
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

    /// Registers `value` on the request root context.
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
        Cx::from_values(
            Arc::new(self.app_context),
            self.request_context.into_values().collect(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextBindingId(u64);

struct ContextBinding {
    id: ContextBindingId,
    value: ContextValue,
}

impl ContextBinding {
    fn new(id: ContextBindingId, value: ContextValue) -> Self {
        Self { id, value }
    }

    fn into_value<T>(binding: Arc<Self>) -> T
    where
        T: Any + Send + Sync,
    {
        let binding = Arc::try_unwrap(binding)
            .expect("request root binding is still shared with a scoped context");
        *binding
            .value
            .downcast::<T>()
            .expect("context binding type changed")
    }
}

impl std::fmt::Debug for ContextBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextBinding")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct RequestState {
    root: BindingMap,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
    next_binding_id: AtomicU64,
}

impl RequestState {
    fn new(values: Vec<ContextValue>) -> Self {
        let mut state = Self {
            root: BindingMap::new(),
            memoize_cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
            next_binding_id: AtomicU64::new(0),
        };
        for value in values {
            let type_id = value.as_ref().type_id();
            let binding_id = state.next_binding_id();
            state
                .root
                .insert(type_id, Arc::new(ContextBinding::new(binding_id, value)));
        }
        state
    }

    fn next_binding_id(&self) -> ContextBindingId {
        let mut id = self.next_binding_id.load(Ordering::Relaxed);
        loop {
            let next = id
                .checked_add(1)
                .expect("request context binding ID overflowed");
            match self.next_binding_id.compare_exchange_weak(
                id,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return ContextBindingId(id),
                Err(current) => id = current,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextRead {
    pub(crate) type_id: TypeId,
    pub(crate) binding_id: Option<ContextBindingId>,
}

pub(crate) struct ContextTracker {
    request_state: Arc<RequestState>,
    input: ContextSnapshot,
    reads: Mutex<Vec<ContextRead>>,
}

impl ContextTracker {
    pub(crate) fn new(cx: &Cx) -> Arc<Self> {
        Arc::new(Self {
            request_state: cx.request_state.clone(),
            input: cx.visible_bindings(),
            reads: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn scope<R>(self: &Arc<Self>, f: impl FnOnce() -> R) -> R {
        ACTIVE_TRACKERS.with(|trackers| trackers.borrow_mut().push(self.clone()));
        let _scope = TrackerScope { tracker: self };
        f()
    }

    pub(crate) fn finish(&self) -> Vec<ContextRead> {
        self.reads.lock().unwrap().clone()
    }

    fn record(&self, read: ContextRead) {
        if self.input.get(&read.type_id).copied() != read.binding_id {
            return;
        }

        let mut reads = self.reads.lock().unwrap();
        if let Some(previous) = reads.iter().find(|item| item.type_id == read.type_id) {
            debug_assert_eq!(*previous, read);
        } else {
            reads.push(read);
        }
    }
}

thread_local! {
    static ACTIVE_TRACKERS: RefCell<Vec<Arc<ContextTracker>>> = const { RefCell::new(Vec::new()) };
}

struct TrackerScope<'a> {
    tracker: &'a Arc<ContextTracker>,
}

impl Drop for TrackerScope<'_> {
    fn drop(&mut self) {
        ACTIVE_TRACKERS.with(|trackers| {
            let active = trackers
                .borrow_mut()
                .pop()
                .expect("context tracker stack underflow");
            debug_assert!(Arc::ptr_eq(&active, self.tracker));
        });
    }
}

fn record_context_read(request_state: &Arc<RequestState>, read: ContextRead) {
    ACTIVE_TRACKERS.with(|trackers| {
        for tracker in trackers.borrow().iter() {
            if Arc::ptr_eq(&tracker.request_state, request_state) {
                tracker.record(read);
            }
        }
    });
}

pub(crate) fn replay_context_reads(cx: &Cx, reads: &[ContextRead]) {
    for &read in reads {
        record_context_read(&cx.request_state, read);
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
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use super::*;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    struct DropCounter(Arc<AtomicUsize>);

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, AtomicOrdering::SeqCst);
        }
    }

    #[test]
    fn root_insert_replaces_and_returns_the_displaced_value() {
        let mut cx = Cx::default();

        assert_eq!(cx.insert(Database("primary")), None);
        let first_id = cx.resolve_binding_id(TypeId::of::<Database>()).unwrap();
        assert_eq!(cx.insert(Database("replica")), Some(Database("primary")));
        let second_id = cx.resolve_binding_id(TypeId::of::<Database>()).unwrap();
        assert_eq!(request_context::<Database>(&cx), &Database("replica"));
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn get_mut_changes_a_root_value() {
        let mut cx = CxTestBuilder::new().request_context(Config(1)).build();

        let first_id = cx.resolve_binding_id(TypeId::of::<Config>()).unwrap();
        cx.get_mut::<Config>().unwrap().0 = 42;
        let second_id = cx.resolve_binding_id(TypeId::of::<Config>()).unwrap();

        assert_eq!(request_context::<Config>(&cx), &Config(42));
        assert_ne!(first_id, second_id);
        let next_id = cx.request_state.next_binding_id.load(Ordering::Relaxed);
        assert_eq!(cx.get_mut::<Database>(), None);
        assert_eq!(
            cx.request_state.next_binding_id.load(Ordering::Relaxed),
            next_id
        );
    }

    #[test]
    fn child_scope_shadows_parent_without_changing_it() {
        let mut cx = Cx::default();
        cx.insert(Database("primary"));
        let child = cx.with(Database("replica"));

        assert_eq!(request_context::<Database>(&child), &Database("replica"));
        assert_eq!(request_context::<Database>(&cx), &Database("primary"));
    }

    #[test]
    fn nearest_scoped_binding_wins() {
        let cx = Cx::default();
        let child = cx.with(Database("replica"));
        let grandchild = child.with(Database("archive"));

        assert_eq!(
            request_context::<Database>(&grandchild),
            &Database("archive")
        );
        assert_eq!(request_context::<Database>(&child), &Database("replica"));
    }

    #[test]
    fn unrelated_values_are_inherited() {
        let cx = CxTestBuilder::new().request_context(Config(42)).build();
        let child = cx.with(Database("replica"));

        assert_eq!(request_context::<Config>(&child), &Config(42));
    }

    #[test]
    fn dropping_a_scope_leaves_the_parent_unchanged() {
        let mut cx = Cx::default();
        {
            let child = cx.with(Database("replica"));
            assert_eq!(request_context::<Database>(&child), &Database("replica"));
        }

        assert_eq!(try_request_context::<Database>(&cx), None);
        cx.insert(Database("primary"));
        assert_eq!(request_context::<Database>(&cx), &Database("primary"));
    }

    #[test]
    fn dropping_a_scope_drops_only_its_shadowing_binding() {
        let parent_drops = Arc::new(AtomicUsize::new(0));
        let child_drops = Arc::new(AtomicUsize::new(0));
        let cx = Cx::default();
        let parent = cx.with(DropCounter(parent_drops.clone()));

        {
            let child = parent.with(DropCounter(child_drops.clone()));
            assert!(Arc::ptr_eq(
                &request_context::<DropCounter>(&child).0,
                &child_drops,
            ));
        }

        assert_eq!(child_drops.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(parent_drops.load(AtomicOrdering::SeqCst), 0);
        drop(parent);
        assert_eq!(parent_drops.load(AtomicOrdering::SeqCst), 1);
    }

    #[test]
    fn with_values_installs_each_tuple_element() {
        let cx = Cx::default();
        let child = cx.with_values((Database("primary"), Config(42)));

        assert_eq!(request_context::<Database>(&child), &Database("primary"));
        assert_eq!(request_context::<Config>(&child), &Config(42));
    }

    #[test]
    fn with_treats_a_tuple_as_one_binding() {
        let cx = Cx::default();
        let child = cx.with((Database("primary"), Config(42)));

        assert_eq!(
            request_context::<(Database, Config)>(&child),
            &(Database("primary"), Config(42))
        );
        assert_eq!(try_request_context::<Database>(&child), None);
    }

    #[test]
    #[should_panic(expected = "duplicate value types")]
    fn with_values_rejects_duplicate_types() {
        let cx = Cx::default();
        let _ = cx.with_values((Config(1), Config(2)));
    }

    #[test]
    fn with_values_supports_arities_two_through_twelve() {
        let cx = Cx::default();
        let _ = cx.with_values((0u8, 1u16));
        let _ = cx.with_values((0u8, 1u16, 2u32));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64));
        let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128));
        let _ = cx.with_values((
            0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128, 10usize,
        ));
        let _ = cx.with_values((
            0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128, 10usize, 11isize,
        ));
    }

    #[test]
    fn each_scope_has_a_fresh_id() {
        let cx = Cx::default();
        let child = cx.with(Config(1));
        let grandchild = child.with(Database("primary"));

        assert_ne!(cx.id(), child.id());
        assert_ne!(child.id(), grandchild.id());
    }

    #[test]
    fn context_types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Cx>();
        assert_send_sync::<CxScope<'static>>();
    }
}
