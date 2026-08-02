mod binding;
mod context_map;
mod id;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashSet,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use bit_set::BitSet;
pub use context_map::*;
pub use id::*;

use crate::{abort::AbortStore, memoize::MemoizeCache};

type ContextValue = Box<dyn Any + Send + Sync>;
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
    bindings: BindingMap,
    binding_mask: binding::Mask,
}

impl Cx {
    /// Creates an empty request context over `app_context`.
    #[doc(hidden)]
    #[must_use]
    pub fn new(app_context: Arc<ContextMap>) -> Self {
        Self::from_values(app_context, std::iter::empty())
    }

    fn from_values(
        app_context: Arc<ContextMap>,
        values: impl IntoIterator<Item = ContextValue>,
    ) -> Self {
        let mut cx = Self {
            id: CxId::new(),
            app_context,
            request_state: Arc::new(RequestState::default()),
            bindings: BindingMap::new(),
            binding_mask: binding::Mask::default(),
        };
        for value in values {
            let _ = cx.install_value(value);
        }
        cx
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
        self.install_value(Box::new(value))
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
        self.assert_request_state_unique();
        let type_id = TypeId::of::<T>();
        let Self {
            request_state,
            bindings,
            binding_mask,
            ..
        } = self;
        let binding = Arc::get_mut(bindings.get_mut(&type_id)?)
            .expect("request root binding is still shared with a scoped context");
        assert!(binding.value.is::<T>(), "context binding type changed");
        let binding_id = binding_mask.install(&request_state.bindings, type_id, Some(binding.id));
        binding.id = binding_id;
        Some(
            binding
                .value
                .downcast_mut::<T>()
                .expect("context binding type changed"),
        )
    }

    fn install_value(&mut self, value: ContextValue) -> Option<Arc<ContextBinding>> {
        let type_id = value.as_ref().type_id();
        let previous_id = self.bindings.get(&type_id).map(|binding| binding.id);
        let binding_id =
            self.binding_mask
                .install(&self.request_state.bindings, type_id, previous_id);
        self.bindings.insert(
            type_id,
            Arc::new(ContextBinding {
                id: binding_id,
                value,
            }),
        )
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
        self.with_erased_values(std::iter::once(Box::new(value) as ContextValue))
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

    fn with_erased_values(&self, values: impl IntoIterator<Item = ContextValue>) -> CxScope<'_> {
        let mut cx = Cx {
            id: CxId::new(),
            app_context: self.app_context.clone(),
            request_state: self.request_state.clone(),
            bindings: self.bindings.clone(),
            binding_mask: self.binding_mask.clone(),
        };
        for value in values {
            let _ = cx.install_value(value);
        }
        CxScope {
            cx,
            parent: PhantomData,
        }
    }

    fn assert_request_state_unique(&mut self) {
        Arc::get_mut(&mut self.request_state)
            .expect("cannot mutate the request root while a scoped context is still reachable");
    }

    pub(crate) fn request_value<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        if let Some(binding) = self.bindings.get(&type_id) {
            record_context_read(&self.request_state, binding.id);
            return Some(
                binding
                    .value
                    .downcast_ref::<T>()
                    .expect("context binding type changed"),
            );
        }

        let binding_id = self.request_state.bindings.root_none(type_id);
        record_context_read(&self.request_state, binding_id);
        None
    }

    #[cfg(test)]
    fn resolve_binding_id(&self, type_id: TypeId) -> binding::Id {
        self.bindings.get(&type_id).map_or_else(
            || self.request_state.bindings.root_none(type_id),
            |binding| binding.id,
        )
    }

    pub(crate) fn context_reads_match(&self, reads: &ContextReadMask) -> bool {
        reads.matches(&self.binding_mask, &self.request_state.bindings)
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
            self.request_context.into_values(),
        )
    }
}

struct ContextBinding {
    id: binding::Id,
    value: ContextValue,
}

impl ContextBinding {
    fn into_value<T>(binding: Arc<Self>) -> T
    where
        T: Any + Send + Sync,
    {
        let binding = Arc::try_unwrap(binding).unwrap_or_else(|_| {
            panic!("request root binding is still shared with a scoped context")
        });
        *binding
            .value
            .downcast::<T>()
            .expect("context binding type changed")
    }
}

#[derive(Debug, Default)]
struct RequestState {
    bindings: binding::Registry,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ContextReadMask {
    bits: BitSet<usize>,
    frontier: usize,
}

impl ContextReadMask {
    fn insert(&mut self, binding_id: binding::Id) {
        self.bits.insert(binding_id.0);
        self.frontier = self.frontier.max(binding_id.frontier());
    }

    fn matches(&self, binding_mask: &binding::Mask, registry: &binding::Registry) -> bool {
        if self.frontier <= binding_mask.frontier {
            self.bits.is_subset(&binding_mask.bits)
        } else {
            self.bits
                .iter()
                .all(|index| binding_mask.effectively_contains(registry, binding::Id(index)))
        }
    }
}

pub(crate) struct ContextTracker {
    request_state: Arc<RequestState>,
    input: binding::Mask,
    reads: Mutex<ContextReadMask>,
}

impl ContextTracker {
    pub(crate) fn new(cx: &Cx) -> Arc<Self> {
        Arc::new(Self {
            request_state: cx.request_state.clone(),
            input: cx.binding_mask.clone(),
            reads: Mutex::new(ContextReadMask::default()),
        })
    }

    pub(crate) fn scope<R>(self: &Arc<Self>, f: impl FnOnce() -> R) -> R {
        ACTIVE_TRACKERS.with(|trackers| trackers.borrow_mut().push(self.clone()));
        let _scope = TrackerScope { tracker: self };
        f()
    }

    pub(crate) fn finish(&self) -> ContextReadMask {
        self.reads.lock().unwrap().clone()
    }

    fn record(&self, binding_id: binding::Id) {
        if !self
            .input
            .effectively_contains(&self.request_state.bindings, binding_id)
        {
            return;
        }
        self.reads.lock().unwrap().insert(binding_id);
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

fn record_context_read(request_state: &Arc<RequestState>, binding_id: binding::Id) {
    ACTIVE_TRACKERS.with(|trackers| {
        for tracker in trackers.borrow().iter() {
            if Arc::ptr_eq(&tracker.request_state, request_state) {
                tracker.record(binding_id);
            }
        }
    });
}

pub(crate) fn replay_context_reads(cx: &Cx, reads: &ContextReadMask) {
    for index in &reads.bits {
        record_context_read(&cx.request_state, binding::Id(index));
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
