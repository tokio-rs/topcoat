mod context_map;
mod id;

pub use context_map::*;
pub use id::*;

use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use crate::{abort::AbortStore, memoize::MemoizeCache};

type ContextValue = Box<dyn Any + Send + Sync>;
type Revision = u64;

const ROOT_SCOPE: ScopeId = ScopeId(0);

/// The context for one request.
///
/// Pages, layouts, components, routes, and layers receive `&Cx` when they need
/// request-scoped information. Use [`app_context`] for values shared by every
/// request, [`request_context`] for values in the current request scope, and
/// [`Cx::with`] to create a child scope that temporarily shadows a value.
pub struct Cx {
    id: CxId,
    app_context: Arc<ContextMap>,
    request: Arc<RequestState>,
    scope: ScopeId,
    memo_frame: Option<Arc<MemoFrame>>,
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
            request: Arc::new(RequestState::new(values)),
            scope: ROOT_SCOPE,
            memo_frame: None,
        }
    }

    /// Returns this context's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.id
    }

    /// Inserts `value` into the current scope and returns the installed value.
    ///
    /// A later insertion of the same type replaces the value returned by
    /// subsequent lookups through this scope and its descendants. References
    /// returned by earlier insertions and lookups remain valid until the
    /// request ends.
    ///
    /// # Panics
    ///
    /// Panics when called from a memoized function body. Create a child scope
    /// with [`with`](Self::with) instead.
    pub fn insert<T>(&self, value: T) -> &T
    where
        T: Any + Send + Sync,
    {
        assert!(
            self.memo_frame.is_none(),
            "cannot insert request context while a memoized function is running"
        );
        let binding = self.request.context.insert(self.scope, Box::new(value));
        self.request.context.value(binding)
    }

    /// Creates a child scope containing `value`.
    ///
    /// The returned context resolves `value` before bindings of the same type
    /// in this context. Other request values remain inherited.
    pub fn with<T>(&self, value: T) -> CxScope<'_>
    where
        T: Any + Send + Sync,
    {
        self.with_erased_values(vec![Box::new(value)])
    }

    /// Creates a child scope containing each value in `values` as a separate
    /// typed binding.
    ///
    /// Use [`with`](Self::with) when a tuple itself is the context value.
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
        let scope = self.request.context.create_scope(self.scope, values);
        CxScope {
            cx: Cx {
                id: CxId::new(),
                app_context: self.app_context.clone(),
                request: self.request.clone(),
                scope,
                memo_frame: self.memo_frame.clone(),
            },
            parent: PhantomData,
        }
    }

    pub(crate) fn request_value<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        let revision = self.memo_revision();
        let resolution = self
            .request
            .context
            .resolve(self.scope, TypeId::of::<T>(), revision);
        let read = match resolution {
            Some(resolution) => ContextRead::Present {
                type_id: TypeId::of::<T>(),
                binding_id: resolution.binding,
            },
            None => ContextRead::Missing {
                type_id: TypeId::of::<T>(),
            },
        };
        self.record_context_read(read, resolution.map(|resolution| resolution.scope));
        resolution.map(|resolution| self.request.context.value(resolution.binding))
    }

    pub(crate) fn memo_revision(&self) -> Revision {
        self.memo_frame
            .as_ref()
            .map_or_else(|| self.request.context.revision(), |frame| frame.revision)
    }

    pub(crate) fn start_memo(&self, revision: Revision) -> Self {
        Self {
            id: self.id,
            app_context: self.app_context.clone(),
            request: self.request.clone(),
            scope: self.scope,
            memo_frame: Some(Arc::new(MemoFrame {
                parent: self.memo_frame.clone(),
                input_scope: self.scope,
                revision,
                reads: Mutex::new(Vec::new()),
            })),
        }
    }

    pub(crate) fn finish_memo(&self) -> Vec<ContextRead> {
        self.memo_frame
            .as_ref()
            .expect("memo context has an active frame")
            .reads
            .lock()
            .unwrap()
            .clone()
    }

    pub(crate) fn context_reads_match(&self, revision: Revision, reads: &[ContextRead]) -> bool {
        self.request
            .context
            .reads_match(self.scope, revision, reads)
    }

    pub(crate) fn record_context_reads(&self, reads: &[ContextRead]) {
        for &read in reads {
            let source_scope = match read {
                ContextRead::Present { binding_id, .. } => {
                    Some(self.request.context.binding_scope(binding_id))
                }
                ContextRead::Missing { .. } => None,
            };
            self.record_context_read(read, source_scope);
        }
    }

    fn record_context_read(&self, read: ContextRead, source_scope: Option<ScopeId>) {
        let Some(frame) = &self.memo_frame else {
            return;
        };
        self.request
            .context
            .record_read(self.scope, frame, read, source_scope);
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
/// It dereferences to [`Cx`] and cannot outlive the context that created it.
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
    fn into_context_values(self) -> Vec<Box<dyn Any + Send + Sync>>;
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
            fn into_context_values(self) -> Vec<Box<dyn Any + Send + Sync>> {
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

#[derive(Debug)]
struct RequestState {
    context: RequestContext,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

impl RequestState {
    fn new(values: Vec<ContextValue>) -> Self {
        Self {
            context: RequestContext::new(values),
            memoize_cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
        }
    }
}

#[derive(Debug)]
struct RequestContext {
    state: Mutex<RequestContextState>,
    bindings: boxcar::Vec<ContextBinding>,
}

impl RequestContext {
    fn new(values: Vec<ContextValue>) -> Self {
        let context = Self {
            state: Mutex::new(RequestContextState {
                revision: 0,
                scopes: vec![Scope::new(None)],
            }),
            bindings: boxcar::Vec::new(),
        };
        context.add_initial_values(ROOT_SCOPE, values);
        context
    }

    fn revision(&self) -> Revision {
        self.state.lock().unwrap().revision
    }

    fn create_scope(&self, parent: ScopeId, values: Vec<ContextValue>) -> ScopeId {
        let scope = {
            let mut state = self.state.lock().unwrap();
            let scope = ScopeId(state.scopes.len());
            state.scopes.push(Scope::new(Some(parent)));
            scope
        };
        self.add_initial_values(scope, values);
        scope
    }

    fn add_initial_values(&self, scope: ScopeId, values: Vec<ContextValue>) {
        let mut state = self.state.lock().unwrap();
        for value in values {
            let type_id = value.as_ref().type_id();
            let binding = ContextBindingId(self.bindings.push(ContextBinding { scope, value }));
            state.scopes[scope.0]
                .bindings
                .entry(type_id)
                .or_default()
                .push(BindingVersion {
                    revision: 0,
                    binding,
                });
        }
    }

    fn insert(&self, scope: ScopeId, value: ContextValue) -> ContextBindingId {
        let mut state = self.state.lock().unwrap();
        let revision = state
            .revision
            .checked_add(1)
            .expect("request context revision overflowed");
        let type_id = value.as_ref().type_id();
        let binding = ContextBindingId(self.bindings.push(ContextBinding { scope, value }));
        state.scopes[scope.0]
            .bindings
            .entry(type_id)
            .or_default()
            .push(BindingVersion { revision, binding });
        state.revision = revision;
        binding
    }

    fn resolve(&self, scope: ScopeId, type_id: TypeId, revision: Revision) -> Option<Resolution> {
        let state = self.state.lock().unwrap();
        state.resolve(scope, type_id, revision)
    }

    fn reads_match(&self, scope: ScopeId, revision: Revision, reads: &[ContextRead]) -> bool {
        let state = self.state.lock().unwrap();
        reads.iter().all(|read| {
            let resolved = state
                .resolve(scope, read.context_type_id(), revision)
                .map(|resolution| resolution.binding);
            match read {
                ContextRead::Present { binding_id, .. } => resolved == Some(*binding_id),
                ContextRead::Missing { .. } => resolved.is_none(),
            }
        })
    }

    fn record_read(
        &self,
        lookup_scope: ScopeId,
        frame: &Arc<MemoFrame>,
        read: ContextRead,
        source_scope: Option<ScopeId>,
    ) {
        let state = self.state.lock().unwrap();
        let mut frame = Some(frame);
        while let Some(current) = frame {
            if state.is_descendant(lookup_scope, current.input_scope)
                && source_scope.is_none_or(|source| {
                    source == current.input_scope
                        || !state.is_descendant(source, current.input_scope)
                })
            {
                current.record(read);
            }
            frame = current.parent.as_ref();
        }
    }

    fn value<T>(&self, binding: ContextBindingId) -> &T
    where
        T: Any + Send + Sync,
    {
        self.bindings[binding.0]
            .value
            .downcast_ref()
            .expect("context binding type changed")
    }

    fn binding_scope(&self, binding: ContextBindingId) -> ScopeId {
        self.bindings[binding.0].scope
    }
}

#[derive(Debug)]
struct RequestContextState {
    revision: Revision,
    scopes: Vec<Scope>,
}

impl RequestContextState {
    fn resolve(
        &self,
        mut scope: ScopeId,
        type_id: TypeId,
        revision: Revision,
    ) -> Option<Resolution> {
        loop {
            let current = &self.scopes[scope.0];
            if let Some(binding) = current.bindings.get(&type_id).and_then(|history| {
                history
                    .iter()
                    .rev()
                    .find(|binding| binding.revision <= revision)
            }) {
                return Some(Resolution {
                    scope,
                    binding: binding.binding,
                });
            }
            scope = current.parent?;
        }
    }

    fn is_descendant(&self, mut scope: ScopeId, ancestor: ScopeId) -> bool {
        loop {
            if scope == ancestor {
                return true;
            }
            let Some(parent) = self.scopes[scope.0].parent else {
                return false;
            };
            scope = parent;
        }
    }
}

#[derive(Debug)]
struct Scope {
    parent: Option<ScopeId>,
    bindings: HashMap<TypeId, Vec<BindingVersion>>,
}

impl Scope {
    fn new(parent: Option<ScopeId>) -> Self {
        Self {
            parent,
            bindings: HashMap::new(),
        }
    }
}

struct ContextBinding {
    scope: ScopeId,
    value: ContextValue,
}

impl std::fmt::Debug for ContextBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextBinding")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BindingVersion {
    revision: Revision,
    binding: ContextBindingId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Resolution {
    scope: ScopeId,
    binding: ContextBindingId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScopeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextBindingId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextRead {
    Present {
        type_id: TypeId,
        binding_id: ContextBindingId,
    },
    Missing {
        type_id: TypeId,
    },
}

impl ContextRead {
    fn context_type_id(self) -> TypeId {
        match self {
            Self::Present { type_id, .. } | Self::Missing { type_id } => type_id,
        }
    }
}

struct MemoFrame {
    parent: Option<Arc<MemoFrame>>,
    input_scope: ScopeId,
    revision: Revision,
    reads: Mutex<Vec<ContextRead>>,
}

impl MemoFrame {
    fn record(&self, read: ContextRead) {
        let mut reads = self.reads.lock().unwrap();
        if let Some(previous) = reads
            .iter()
            .find(|previous| previous.context_type_id() == read.context_type_id())
        {
            debug_assert_eq!(*previous, read);
        } else {
            reads.push(read);
        }
    }
}

#[inline]
#[doc(hidden)]
#[must_use]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.request.memoize_cache
}

#[inline]
#[doc(hidden)]
#[must_use]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.request.abort_store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    #[test]
    fn insert_returns_stable_reference_and_replaces_current_binding() {
        let cx = Cx::default();
        let first = cx.insert(Database("primary"));
        let second = cx.insert(Database("replica"));

        assert_eq!(first, &Database("primary"));
        assert_eq!(second, &Database("replica"));
        assert_eq!(request_context::<Database>(&cx), &Database("replica"));
    }

    #[test]
    fn child_scope_shadows_parent_without_changing_it() {
        let cx = Cx::default();
        cx.insert(Database("primary"));
        let child = cx.with(Database("replica"));

        assert_eq!(request_context::<Database>(&child), &Database("replica"));
        assert_eq!(request_context::<Database>(&cx), &Database("primary"));
    }

    #[test]
    fn child_scope_inherits_later_parent_insertions() {
        let cx = Cx::default();
        let child = cx.with(Config(1));
        cx.insert(Database("primary"));

        assert_eq!(request_context::<Database>(&child), &Database("primary"));
        assert_eq!(request_context::<Config>(&child), &Config(1));
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
    fn inserting_into_child_does_not_change_parent_or_sibling() {
        let cx = Cx::default();
        let left = cx.with(Config(1));
        let right = cx.with(Config(2));
        left.insert(Database("left"));

        assert_eq!(try_request_context::<Database>(&cx), None);
        assert_eq!(try_request_context::<Database>(&right), None);
        assert_eq!(request_context::<Database>(&left), &Database("left"));
    }

    #[test]
    fn parent_insertion_does_not_override_child_binding() {
        let cx = Cx::default();
        let child = cx.with(Database("child"));
        cx.insert(Database("root"));

        assert_eq!(request_context::<Database>(&child), &Database("child"));
        assert_eq!(request_context::<Database>(&cx), &Database("root"));
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
}
