mod context_map;
mod id;
mod lookup;
mod tracking;

use std::{
    any::{Any, TypeId},
    iter,
    panic::Location,
    sync::Arc,
};

pub use context_map::*;
pub use id::*;
pub use lookup::*;
pub(crate) use tracking::*;

pub use crate::memoize::MemoizeAsRef;
use crate::{
    abort::AbortStore,
    identity::{AmbiguousIdentityError, Identity, IdentityKey, SiteKey},
    memoize::MemoizeCache,
};

/// The request context.
///
/// Add `cx: &Cx` to a handler or component to access context. Topcoat supplies
/// it automatically. Read values with [`context`] and [`try_context`], whether
/// they were registered on the router or added during the request.
///
/// Clones share the request's state. Use [`with`](Self::with) or
/// [`with_many`](Self::with_many) to add context values in a child scope
/// without changing the parent. Move an owned clone into work that outlives
/// the handler, such as a streaming response.
#[derive(Debug, Default, Clone)]
pub struct Cx {
    /// The values visible to this handle and the request they belong to.
    scope: Arc<Scope>,
    /// The tracker recording this handle's context reads, if one is
    /// installed.
    tracker: Option<Arc<ContextTracker>>,
    /// The identity of this handle's scope.
    identity: Identity,
}

impl Cx {
    /// Creates the context for one request over `shared`, the values shared by
    /// every request, such as those registered on a router.
    ///
    /// `shared` is never copied. Values added with [`with`](Self::with)
    /// shadow its values for this request only.
    #[must_use]
    pub fn new(shared: Arc<ContextMap>) -> Self {
        Self {
            scope: Arc::new(Scope {
                request: Arc::new(RequestState {
                    id: CxId::new(),
                    memoize_cache: MemoizeCache::new(),
                    abort_store: AbortStore::new(),
                }),
                values: ContextMap::new(),
                sealed: Some(Arc::new(Sealed {
                    values: shared,
                    parent: None,
                })),
            }),
            tracker: None,
            identity: Identity::ROOT,
        }
    }

    /// Returns this request's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.scope.request.id
    }

    /// Returns a child context with an identity derived from the parent's identity,
    /// this call's source location, and `key`.
    ///
    /// Use `()` to distinguish separate call locations, or an item key to
    /// distinguish repeated calls at one location:
    ///
    /// ```
    /// # use topcoat_core::context::Cx;
    /// # let cx = Cx::default();
    /// let first = cx.keyed(());
    /// let second = cx.keyed(());
    ///
    /// for id in [1, 2, 3] {
    ///     let child = cx.keyed(id);
    /// }
    /// ```
    ///
    /// The same inputs produce the same key. Moving the call in source
    /// changes it. The child shares the parent's request state and context
    /// values.
    #[must_use]
    #[track_caller]
    pub fn keyed(&self, key: impl IdentityKey) -> Self {
        let site = SiteKey::from_location(Location::caller());
        Self {
            identity: self.identity.keyed_child(site, key),
            ..self.clone()
        }
    }

    /// Returns the values visible to this handle.
    #[inline]
    pub(crate) fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns the tracker recording this handle's context reads, if one is
    /// installed.
    #[inline]
    pub(crate) fn tracker(&self) -> Option<&ContextTracker> {
        self.tracker.as_deref()
    }

    /// Returns a child handle whose context also holds `value`.
    ///
    /// The child inherits other context values and shares the request's state.
    /// If this type is already registered, the child sees `value` while the
    /// parent still sees the original.
    #[must_use]
    pub fn with<T>(&self, value: T) -> Cx
    where
        T: Any + Send + Sync,
    {
        let mut values = self.scope.values.clone();
        values.insert(value);
        self.derive(values)
    }

    /// Returns a child handle whose context also holds every value in
    /// `values`, a tuple of context values or a [`ContextMap`].
    ///
    /// Behaves like chained [`with`](Self::with) calls, but builds the child's
    /// context in one step.
    #[must_use]
    pub fn with_many<V>(&self, values: V) -> Cx
    where
        V: ContextValues,
    {
        let mut derived = self.scope.values.clone();
        values.install(&mut derived);
        self.derive(derived)
    }

    /// Wraps derived request values into a child handle sharing this
    /// request's state and sealed layers.
    fn derive(&self, values: ContextMap) -> Cx {
        Cx {
            scope: Arc::new(Scope {
                request: Arc::clone(&self.scope.request),
                values,
                sealed: self.scope.sealed.clone(),
            }),
            tracker: self.tracker.clone(),
            identity: self.identity,
        }
    }

    /// Returns a child handle whose context reads are recorded, along with the
    /// tracker collecting them.
    ///
    /// The child shares this handle's scope, and that scope is also the
    /// tracker's entry scope. A tracker inherited from an enclosing `track`
    /// call is replaced, not stacked: reads made through the child and its
    /// descendants are recorded by the new tracker only.
    pub(crate) fn track(&self) -> (Cx, Arc<ContextTracker>) {
        let tracker = Arc::new(ContextTracker::new(Arc::clone(&self.scope)));
        let child = Cx {
            scope: Arc::clone(&self.scope),
            tracker: Some(Arc::clone(&tracker)),
            identity: self.identity,
        };
        (child, tracker)
    }
}

/// The values visible to the handles of one context scope.
///
/// Values added with [`Cx::with`] live in `values`, which each `with` copies.
/// Values shared beyond one request, such as those registered on a router,
/// live in sealed layers that are never copied.
#[derive(Debug, Default)]
pub(crate) struct Scope {
    /// The state of the request this scope belongs to.
    request: Arc<RequestState>,
    /// The values added to this request.
    values: ContextMap,
    /// The newest sealed layer, linking to the layers beneath it.
    sealed: Option<Arc<Sealed>>,
}

impl Scope {
    /// Returns the value of type `T` visible in this scope together with the
    /// id of the binding holding it, or `None` if no such value is registered.
    ///
    /// Values added to the request shadow those of the sealed layers, and
    /// newer sealed layers shadow older ones.
    pub(crate) fn lookup<T>(&self) -> Option<(BindingId, &T)>
    where
        T: Any + Send + Sync,
    {
        self.values
            .lookup::<T>()
            .or_else(|| self.sealed().find_map(ContextMap::lookup::<T>))
    }

    /// Returns the id of the binding that a read of `type_id` resolves to in
    /// this scope, or `None` if the type is not registered.
    ///
    /// Resolves like [`lookup`](Self::lookup), but from a runtime [`TypeId`],
    /// so a recorded read can be revalidated without naming its type.
    pub(crate) fn binding_id(&self, type_id: TypeId) -> Option<BindingId> {
        self.values
            .binding_id(type_id)
            .or_else(|| self.sealed().find_map(|values| values.binding_id(type_id)))
    }

    /// Returns the sealed layers' values, newest first.
    fn sealed(&self) -> impl Iterator<Item = &ContextMap> {
        iter::successors(self.sealed.as_deref(), |layer| layer.parent.as_deref())
            .map(|layer| &*layer.values)
    }
}

/// One layer of values shared beyond a single request.
///
/// Each request links its own node to the shared values, so deriving a scope
/// only touches the reference count of this request's node, never one that
/// other requests share.
#[derive(Debug)]
struct Sealed {
    /// The values of this layer.
    values: Arc<ContextMap>,
    /// The layer beneath this one.
    parent: Option<Arc<Sealed>>,
}

/// The state shared by every handle to one request's [`Cx`].
#[derive(Debug, Default)]
struct RequestState {
    id: CxId,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

/// Returns the identity of this context's scope.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call or an enclosing scope introduced
/// ambiguity. Use [`try_identity`] when an identity may be ambiguous.
#[must_use]
#[track_caller]
pub fn identity(cx: &Cx) -> Identity {
    match try_identity(cx) {
        Ok(identity) => identity,
        Err(error) => panic!("{error}"),
    }
}

/// Returns this context's identity, or the ambiguity inherited by it.
///
/// # Errors
///
/// Returns an error naming the scope that introduced ambiguity.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call. To memoize a value that depends
/// on identity, read the identity before the call and pass it as an argument.
#[track_caller]
pub fn try_identity(cx: &Cx) -> Result<Identity, AmbiguousIdentityError> {
    assert!(
        cx.tracker.is_none(),
        "identity cannot be read inside memoized functions"
    );
    cx.identity.checked()
}

/// Reads the identity for derivation without checking ambiguity.
#[doc(hidden)]
#[must_use]
pub fn identity_raw(cx: &Cx) -> Identity {
    cx.identity
}

/// Sets the identity of an owned context.
#[doc(hidden)]
#[must_use]
pub fn with_identity(cx: Cx, identity: Identity) -> Cx {
    Cx { identity, ..cx }
}

#[inline]
#[must_use]
#[doc(hidden)]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.scope.request.memoize_cache
}

#[inline]
#[must_use]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.scope.request.abort_store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Marker(u32);

    #[derive(Debug, PartialEq)]
    struct Other(&'static str);

    /// Returns a handle whose sealed layer holds `value`.
    fn shared<T: Any + Send + Sync>(value: T) -> Cx {
        let mut values = ContextMap::new();
        values.insert(value);
        Cx::new(Arc::new(values))
    }

    #[test]
    fn a_fresh_context_has_a_unique_id() {
        let first = Cx::new(Arc::new(ContextMap::new()));
        let second = Cx::new(Arc::new(ContextMap::new()));
        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn keys_distinguish_locations_and_repetitions() {
        fn child(cx: &Cx, key: u32) -> Cx {
            cx.keyed(key)
        }

        let cx = Cx::default();
        assert_ne!(identity(&cx.keyed(())), identity(&cx.keyed(())));
        assert_eq!(
            identity(&child(&cx, 1)),
            identity(&child(&Cx::default(), 1))
        );
        assert_ne!(identity(&child(&cx, 1)), identity(&child(&cx, 2)));
        assert_ne!(
            identity(&child(&child(&cx, 1), 2)),
            identity(&child(&cx, 2))
        );
        assert_eq!(identity(&cx), Identity::ROOT);
    }

    #[test]
    fn keyed_contexts_share_request_state_and_preserve_their_scope() {
        let cx = Cx::default().with(Marker(7));
        let child = cx.keyed("child");
        assert!(Arc::ptr_eq(&child.scope, &cx.scope));
        assert_eq!(child.id(), cx.id());
        assert!(std::ptr::eq(memoize_cache(&child), memoize_cache(&cx)));
        assert!(std::ptr::eq(
            context::<Marker>(&child),
            context::<Marker>(&cx)
        ));

        let expected = identity(&child);
        assert_eq!(identity(&child.clone()), expected);
        assert_eq!(identity(&child.with(Other("value"))), expected);
        assert_eq!(identity(&child.with_many((Other("value"),))), expected);
        assert_eq!(identity_raw(&child.track().0), expected);
    }

    #[test]
    #[should_panic(expected = "identity cannot be read inside memoized functions")]
    fn memoized_functions_cannot_read_identity() {
        let cx = Cx::default();
        memoize_cache(&cx).memoize(&cx, (), (), |cx, ()| {
            identity(&cx.keyed(()).with(Marker(7)))
        });
    }

    #[test]
    #[should_panic(expected = "identity cannot be read inside memoized functions")]
    fn memoized_functions_cannot_try_identity() {
        let cx = Cx::default();
        memoize_cache(&cx).memoize(&cx, (), (), |cx, ()| try_identity(&cx));
    }

    #[tokio::test]
    #[should_panic(expected = "identity cannot be read inside memoized functions")]
    async fn memoized_functions_cannot_read_identity_after_suspension() {
        let cx = Cx::default();
        memoize_cache(&cx)
            .memoize_async(&cx, (), (), |cx, ()| async move {
                tokio::task::yield_now().await;
                identity(&cx)
            })
            .await;
    }

    #[test]
    fn with_registers_a_value_on_the_child() {
        let cx = Cx::default();
        let child = cx.with(Marker(1));

        assert_eq!(try_context::<Marker>(&cx), None);
        assert_eq!(context::<Marker>(&child), &Marker(1));
    }

    #[test]
    fn with_shadows_without_touching_the_parent() {
        let cx = Cx::default().with(Marker(1));
        let child = cx.with(Marker(2));

        assert_eq!(context::<Marker>(&cx), &Marker(1));
        assert_eq!(context::<Marker>(&child), &Marker(2));
    }

    #[test]
    fn a_child_inherits_the_parent_context() {
        let cx = shared(Other("app")).with(Marker(7));
        let child = cx.with(Other("request"));

        assert_eq!(context::<Marker>(&child), &Marker(7));
        assert_eq!(context::<Other>(&child), &Other("request"));
        assert_eq!(context::<Other>(&cx), &Other("app"));
    }

    #[test]
    fn with_many_registers_every_value() {
        let cx = Cx::default().with_many((Marker(1), Other("many")));

        assert_eq!(context::<Marker>(&cx), &Marker(1));
        assert_eq!(context::<Other>(&cx), &Other("many"));
    }

    #[test]
    fn a_child_shares_the_request_state() {
        let cx = Cx::default();
        let child = cx.with(Marker(1));

        assert_eq!(child.id(), cx.id());
        assert!(std::ptr::eq(memoize_cache(&child), memoize_cache(&cx)));
        assert!(std::ptr::eq(abort_store(&child), abort_store(&cx)));
    }

    #[test]
    fn clones_outlive_the_original() {
        let cx = Cx::default().with(Marker(7));
        let id = cx.id();
        let handle = cx.clone();
        drop(cx);

        assert_eq!(context::<Marker>(&handle).0, 7);
        assert_eq!(handle.id(), id);
    }

    #[test]
    fn with_shares_sealed_values_instead_of_copying_them() {
        let cx = shared(Other("app"));
        let child = cx.with(Marker(1)).with_many((Marker(2),));

        assert!(std::ptr::eq(
            context::<Other>(&child),
            context::<Other>(&cx)
        ));
    }

    #[test]
    fn reads_of_sealed_values_are_tracked() {
        let cx = shared(Other("app"));
        let (child, tracker) = cx.track();

        let _ = context::<Other>(&child);

        let reads = tracker.take_reads();
        assert_eq!(reads.len(), 1);
        assert!(reads[0].matches(cx.scope()));
        assert!(!reads[0].matches(cx.with(Other("request")).scope()));
    }

    #[test]
    fn track_shares_the_scope() {
        let cx = Cx::default().with(Marker(1));
        let (child, _tracker) = cx.track();

        assert!(Arc::ptr_eq(&child.scope, &cx.scope));
        assert!(cx.tracker.is_none());
        assert!(child.tracker.is_some());
    }

    #[test]
    fn derived_handles_keep_the_tracker() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();
        let derived = child.with(Marker(1)).keyed(());

        assert!(std::ptr::eq(
            derived.tracker().expect("tracker is kept"),
            &*tracker
        ));
    }

    #[test]
    fn handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Cx>();
    }
}
