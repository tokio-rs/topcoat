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

/// Access to the values and state of one request.
///
/// Give a handler or component a `cx: &Cx` parameter and Topcoat supplies it.
/// Use [`context`] or [`try_context`] to read values added on the router or
/// while handling the request.
///
/// Use [`with`](Self::with) or [`with_many`](Self::with_many) to create a
/// child context with extra values. The parent stays unchanged. Cloning a
/// `Cx` shares its state and values. Move a clone into a stream or task that
/// needs to keep them after the handler returns.
#[derive(Debug, Default, Clone)]
pub struct Cx {
    /// This handle's values and shared request state.
    scope: Arc<Scope>,
    /// Records context reads when this handle is used in a tracked call.
    tracker: Option<Arc<ContextTracker>>,
    /// The identity of this handle's scope.
    identity: Identity,
}

impl Cx {
    /// Starts a new request context with no values.
    ///
    /// Add values shared across requests with
    /// [`with_shared`](Self::with_shared), and values of this request with
    /// [`with`](Self::with).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns this request's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.scope.request.id
    }

    /// Creates a child identity from this context's identity, the location
    /// of the call in the source, and `key`.
    ///
    /// Pass `()` when the call location is enough to tell children apart.
    /// For repeated calls at one location, pass a key for each item:
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
    /// The same inputs give the same identity. Moving the call to another
    /// source location changes it. The child keeps the parent's request
    /// state and values.
    #[must_use]
    #[track_caller]
    pub fn keyed(&self, key: impl IdentityKey) -> Self {
        let site = SiteKey::from_location(Location::caller());
        Self {
            identity: self.identity.keyed_child(site, key),
            ..self.clone()
        }
    }

    /// Returns the scope this handle reads from.
    #[inline]
    pub(crate) fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns this handle's read tracker, if tracking is enabled.
    #[inline]
    pub(crate) fn tracker(&self) -> Option<&ContextTracker> {
        self.tracker.as_deref()
    }

    /// Creates a child context containing `value`.
    ///
    /// In the child, `value` shadows any earlier value of the same type.
    /// Other values and the request state are shared with the parent.
    /// The parent's values stay unchanged.
    #[must_use]
    pub fn with<T>(&self, value: T) -> Cx
    where
        T: Any + Send + Sync,
    {
        let mut values = self.scope.values.clone();
        values.insert(value);
        self.derive(values)
    }

    /// Creates a child context with all the supplied values.
    ///
    /// Pass a tuple or a [`ContextMap`]. This has the same result as calling
    /// [`with`](Self::with) for each value, but creates the child in one step.
    #[must_use]
    pub fn with_many<V>(&self, values: V) -> Cx
    where
        V: ContextValues,
    {
        let mut derived = self.scope.values.clone();
        values.install(&mut derived);
        self.derive(derived)
    }

    /// Creates a child context that can also read the values of `shared`.
    ///
    /// Several requests can use the same map without copying its values.
    /// The child searches `shared` after the values added with
    /// [`with`](Self::with) or [`with_many`](Self::with_many), whatever order
    /// the calls were made in, and before the maps added by earlier calls to
    /// this method. A request value therefore shadows a shared value of the
    /// same type, and the newest shared map shadows older ones.
    #[must_use]
    pub fn with_shared(&self, shared: Arc<ContextMap>) -> Cx {
        Cx {
            scope: Arc::new(Scope {
                request: Arc::clone(&self.scope.request),
                values: self.scope.values.clone(),
                sealed: Some(Arc::new(Sealed {
                    id: BindingId::new(),
                    values: shared,
                    parent: self.scope.sealed.clone(),
                })),
            }),
            tracker: self.tracker.clone(),
            identity: self.identity,
        }
    }

    /// Creates a child with `values`, keeping the same request state and
    /// sealed layers.
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

    /// Creates a handle with read tracking and returns its tracker too.
    ///
    /// The handle uses the same scope. The tracker saves that scope as the
    /// starting point for checking dependencies. Reads through the new
    /// handle or its children go only to this tracker, even if the parent
    /// already had one.
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

/// The values and request state used by a context handle.
///
/// [`Cx::with`] copies the `values` map when creating a child. Values shared
/// across requests live in sealed layers, which the child shares without
/// copying their maps.
#[derive(Debug, Default)]
pub(crate) struct Scope {
    /// State shared by all scopes of this request.
    request: Arc<RequestState>,
    /// Values added while handling the request.
    values: ContextMap,
    /// The first sealed layer to search, with links to the older layers.
    sealed: Option<Arc<Sealed>>,
}

impl Scope {
    /// Finds `T` and returns its binding ID and a reference to its value.
    /// Returns `None` if the scope has no value of that type.
    ///
    /// Search request values first, then sealed layers from newest to oldest.
    pub(crate) fn lookup<T>(&self) -> Option<(BindingId, &T)>
    where
        T: Any + Send + Sync,
    {
        self.values
            .lookup::<T>()
            .or_else(|| self.sealed().find_map(ContextMap::lookup::<T>))
    }

    /// Visits every value of type `T`, closest first.
    ///
    /// Uses the same search order as [`lookup`](Self::lookup) but continues
    /// past the first value. Request values come from newest to oldest, then
    /// the values of each sealed layer from newest to oldest.
    pub(crate) fn iter<T>(&self) -> impl Iterator<Item = &T>
    where
        T: Any + Send + Sync,
    {
        self.values
            .iter::<T>()
            .chain(self.sealed().flat_map(ContextMap::iter::<T>))
    }

    /// Finds the binding ID for `type_id`, or returns `None` if it is absent.
    ///
    /// Uses the same search order as [`lookup`](Self::lookup). Accepting a
    /// [`TypeId`] lets tracking check a saved read without a type parameter.
    pub(crate) fn binding_id(&self, type_id: TypeId) -> Option<BindingId> {
        self.values
            .binding_id(type_id)
            .or_else(|| self.sealed().find_map(|values| values.binding_id(type_id)))
    }

    /// Returns the binding ID of the newest request value of `type_id`, or
    /// `None` if the request values have none.
    ///
    /// The ID also identifies every older request value of that type.
    pub(crate) fn request_binding_id(&self, type_id: TypeId) -> Option<BindingId> {
        self.values.binding_id(type_id)
    }

    /// Returns the ID of the newest sealed layer, or `None` if there is none.
    ///
    /// Layers never change after they are created, so the ID also
    /// identifies every older layer.
    pub(crate) fn sealed_id(&self) -> Option<BindingId> {
        self.sealed.as_ref().map(|layer| layer.id)
    }

    /// Visits the sealed maps from newest to oldest.
    fn sealed(&self) -> impl Iterator<Item = &ContextMap> {
        iter::successors(self.sealed.as_deref(), |layer| layer.parent.as_deref())
            .map(|layer| &*layer.values)
    }
}

/// A link to a map of values shared across requests.
///
/// Each request owns its link to the shared map. Creating a child scope
/// updates the reference count of that link without touching the shared
/// map's reference count.
#[derive(Debug)]
struct Sealed {
    /// Identifies this layer together with its parents.
    id: BindingId,
    /// The map this layer reads from.
    values: Arc<ContextMap>,
    /// The next layer to search if this one lacks a type.
    parent: Option<Arc<Sealed>>,
}

/// State kept for one request and shared by all its [`Cx`] handles.
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

    /// Creates a shared map holding `value`.
    fn map<T: Any + Send + Sync>(value: T) -> Arc<ContextMap> {
        let mut values = ContextMap::new();
        values.insert(value);
        Arc::new(values)
    }

    /// Creates a context with `value` in its shared map.
    fn shared<T: Any + Send + Sync>(value: T) -> Cx {
        Cx::new().with_shared(map(value))
    }

    /// Creates a context whose sealed layers hold `layers`, newest first.
    fn sealed_layers(layers: Vec<ContextMap>) -> Cx {
        let sealed = layers.into_iter().rev().fold(None, |parent, values| {
            Some(Arc::new(Sealed {
                id: BindingId::new(),
                values: Arc::new(values),
                parent,
            }))
        });
        Cx {
            scope: Arc::new(Scope {
                sealed,
                ..Scope::default()
            }),
            ..Cx::default()
        }
    }

    /// Collects every `Marker` that `cx` can see, closest first.
    fn markers(cx: &Cx) -> Vec<u32> {
        cx.scope.iter::<Marker>().map(|marker| marker.0).collect()
    }

    #[test]
    fn a_fresh_context_has_a_unique_id() {
        let first = Cx::new();
        let second = Cx::new();
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
        let cx = shared(Other("router")).with(Marker(7));
        let child = cx.with(Other("scoped"));

        assert_eq!(context::<Marker>(&child), &Marker(7));
        assert_eq!(context::<Other>(&child), &Other("scoped"));
        assert_eq!(context::<Other>(&cx), &Other("router"));
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
        let cx = shared(Other("router"));
        let child = cx.with(Marker(1)).with_many((Marker(2),));

        assert!(std::ptr::eq(
            context::<Other>(&child),
            context::<Other>(&cx)
        ));
    }

    #[test]
    fn request_values_shadow_shared_values_added_later() {
        let cx = Cx::new().with(Marker(1)).with_shared(map(Marker(2)));

        assert_eq!(context::<Marker>(&cx), &Marker(1));
        assert_eq!(markers(&cx), [1, 2]);
    }

    #[test]
    fn newer_shared_maps_shadow_older_ones() {
        let cx = shared(Marker(1)).with_shared(map(Marker(2)));

        assert_eq!(context::<Marker>(&cx), &Marker(2));
        assert_eq!(markers(&cx), [2, 1]);
    }

    #[test]
    fn with_shared_keeps_the_request_state() {
        let cx = Cx::new().with(Marker(1));
        let child = cx.with_shared(map(Other("shared")));

        assert_eq!(child.id(), cx.id());
        assert!(std::ptr::eq(memoize_cache(&child), memoize_cache(&cx)));
        assert_eq!(try_context::<Other>(&cx), None);
    }

    #[test]
    fn iter_visits_request_values_before_sealed_values() {
        let cx = shared(Marker(1))
            .with(Marker(2))
            .with_many((Marker(3), Other("many")));

        assert_eq!(markers(&cx), [3, 2, 1]);
    }

    #[test]
    fn iter_visits_sealed_layers_newest_first() {
        let mut older = ContextMap::new();
        older.insert(Marker(0));
        older.insert(Marker(1));
        let mut newer = ContextMap::new();
        newer.insert(Marker(2));
        newer.insert(Other("newer"));
        let cx = sealed_layers(vec![newer, ContextMap::new(), older]).with(Marker(3));

        assert_eq!(markers(&cx), [3, 2, 1, 0]);
    }

    #[test]
    fn iter_leaves_the_parent_unchanged() {
        let cx = shared(Marker(1)).with(Marker(2));
        let child = cx.with(Marker(3));

        assert_eq!(markers(&cx), [2, 1]);
        assert_eq!(markers(&child), [3, 2, 1]);
    }

    #[test]
    fn iter_is_empty_for_unregistered_type() {
        let cx = shared(Other("router")).with(Other("request"));

        assert_eq!(markers(&cx), Vec::<u32>::new());
    }

    #[test]
    fn reads_of_sealed_values_are_tracked() {
        let cx = shared(Other("router"));
        let (child, tracker) = cx.track();

        let _ = context::<Other>(&child);

        let reads = tracker.take_reads();
        assert_eq!(reads.len(), 1);
        assert!(reads[0].matches(cx.scope()));
        assert!(!reads[0].matches(cx.with(Other("scoped")).scope()));
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

        assert!(Arc::ptr_eq(
            derived.tracker.as_ref().expect("tracker is kept"),
            &tracker
        ));
    }

    #[test]
    fn handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Cx>();
    }
}
