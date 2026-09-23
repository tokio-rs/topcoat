//! The request context [`Cx`] and the app and request context values it
//! gives access to.

mod app_context;
mod id;
mod request_context;
mod tracking;

use std::{any::Any, panic::Location, sync::Arc};

pub use app_context::*;
pub use id::*;
pub use request_context::*;
pub(crate) use tracking::*;

pub use crate::memoize::MemoizeAsRef;
use crate::{
    abort::AbortStore,
    identity::{AmbiguousIdentityError, Identity, IdentityKey, SiteKey},
    memoize::MemoizeCache,
};

/// The request context.
///
/// Pages, layouts, components, and routes can take a `cx: &Cx` parameter when
/// they need request-scoped information, and Topcoat passes it in for them.
/// Pass it on to helper functions that need it. Read values registered for
/// the whole application with [`app_context`] and values registered for the
/// request with [`request_context`].
///
/// A `Cx` is a handle to state shared by everything serving the same request.
/// [`with`](Self::with) and [`with_many`](Self::with_many) derive a child
/// handle whose request context holds additional values, leaving the parent
/// unchanged. Cloning a handle is cheap. Work that outlives the handler, such
/// as a streaming response body or a WebSocket task, should move an owned
/// clone into the work.
///
/// `Cx::default()` creates a context with empty app and request contexts,
/// which is handy in tests. Use [`CxTestBuilder`] to create one with values.
#[derive(Debug, Default, Clone)]
pub struct Cx {
    /// The state shared by handles in the same context scope.
    state: Arc<CxState>,
    /// The identity of this handle's scope.
    identity: Identity,
}

impl Cx {
    /// Creates the context for a new request that shares `app_context`, with
    /// an empty request context.
    #[must_use]
    pub fn new(app_context: Arc<AppContext>) -> Self {
        Self::from_parts(app_context, RequestContext::new())
    }

    /// Creates a `Cx` from the given app and request contexts.
    fn from_parts(app_context: Arc<AppContext>, request_context: RequestContext) -> Self {
        Self {
            state: Arc::new(CxState {
                shared: Arc::new(RequestShared {
                    id: CxId::new(),
                    app_context,
                    memoize_cache: MemoizeCache::new(),
                    abort_store: AbortStore::new(),
                }),
                request_context: Arc::new(request_context),
                tracker: None,
            }),
            identity: Identity::ROOT,
        }
    }

    /// Returns the [`CxId`] of the request this handle belongs to.
    ///
    /// Every handle derived from the same request, including clones and
    /// children, returns the same id.
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.state.shared.id
    }

    /// Returns a child context whose [`Identity`] is derived from this
    /// context's identity, the source location of this call, and `key`.
    ///
    /// Pass `()` when the source location alone tells calls apart, or a key
    /// such as an item id to tell apart repeated calls at one location:
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
    /// The same inputs always produce the same identity. Moving the call to
    /// another place in the source changes it. The child shares the parent's
    /// request state and context values.
    #[must_use]
    #[track_caller]
    pub fn keyed(&self, key: impl IdentityKey) -> Self {
        let site = SiteKey::from_location(Location::caller());
        Self {
            identity: self.identity.keyed_child(site, key),
            ..self.clone()
        }
    }

    /// Returns the request context visible to this handle's scope.
    #[inline]
    pub(crate) fn request_context(&self) -> &RequestContext {
        &self.state.request_context
    }

    /// Returns the tracker recording this handle's request context reads, if
    /// one is installed.
    #[inline]
    pub(crate) fn tracker(&self) -> Option<&ContextTracker> {
        self.state.tracker.as_deref()
    }

    /// Returns a child handle whose request context also holds `value`.
    ///
    /// The child inherits every other request context value and shares the
    /// rest of the request state, such as the app context and the memoize
    /// cache, with `self`. If a value of the same type is already present,
    /// the child shadows it: lookups through the child see `value`, while
    /// lookups through `self` still see the original.
    ///
    /// ```
    /// use topcoat::context::{Cx, request_context};
    ///
    /// struct Locale(&'static str);
    ///
    /// let cx = Cx::default().with(Locale("en"));
    /// let child = cx.with(Locale("de"));
    ///
    /// assert_eq!(request_context::<Locale>(&cx).0, "en");
    /// assert_eq!(request_context::<Locale>(&child).0, "de");
    /// ```
    #[must_use]
    pub fn with<T>(&self, value: T) -> Cx
    where
        T: Any + Send + Sync,
    {
        let mut request_context = (*self.state.request_context).clone();
        request_context.insert(value);
        self.scope(request_context)
    }

    /// Returns a child handle whose request context also holds every value in
    /// `values`, which is either a tuple of values or a [`RequestContext`].
    ///
    /// This behaves like chained [`with`](Self::with) calls, but builds the
    /// child in one step.
    #[must_use]
    pub fn with_many<V>(&self, values: V) -> Cx
    where
        V: ContextValues,
    {
        let mut request_context = (*self.state.request_context).clone();
        values.install(&mut request_context);
        self.scope(request_context)
    }

    /// Wraps a derived request context into a child handle sharing this
    /// request's state.
    fn scope(&self, request_context: RequestContext) -> Cx {
        Cx {
            state: Arc::new(CxState {
                shared: Arc::clone(&self.state.shared),
                request_context: Arc::new(request_context),
                tracker: self.state.tracker.clone(),
            }),
            identity: self.identity,
        }
    }

    /// Returns a child handle whose request context reads are recorded, along
    /// with the tracker collecting them.
    ///
    /// The child shares this handle's scope, and that scope is also the
    /// tracker's entry scope. A tracker inherited from an enclosing `track`
    /// call is replaced, not stacked: reads made through the child and its
    /// descendants are recorded by the new tracker only.
    pub(crate) fn track(&self) -> (Cx, Arc<ContextTracker>) {
        let tracker = Arc::new(ContextTracker::new(Arc::clone(&self.state.request_context)));
        let child = Cx {
            state: Arc::new(CxState {
                shared: Arc::clone(&self.state.shared),
                request_context: Arc::clone(&self.state.request_context),
                tracker: Some(Arc::clone(&tracker)),
            }),
            identity: self.identity,
        };
        (child, tracker)
    }
}

/// The bindings and tracker shared by handles in one context scope.
#[derive(Debug, Default)]
struct CxState {
    shared: Arc<RequestShared>,
    request_context: Arc<RequestContext>,
    tracker: Option<Arc<ContextTracker>>,
}

/// The state shared by every handle to one request's [`Cx`].
#[derive(Debug, Default)]
struct RequestShared {
    id: CxId,
    app_context: Arc<AppContext>,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

/// Builds a [`Cx`] with app and request context values, for tests.
///
/// ```
/// use topcoat::context::{CxTestBuilder, app_context, request_context};
///
/// struct Config(u32);
/// struct UserId(u64);
///
/// let cx = CxTestBuilder::new()
///     .app_context(Config(3))
///     .request_context(UserId(42))
///     .build();
///
/// assert_eq!(app_context::<Config>(&cx).0, 3);
/// assert_eq!(request_context::<UserId>(&cx).0, 42);
/// ```
#[derive(Debug, Default)]
pub struct CxTestBuilder {
    app_context: AppContext,
    request_context: RequestContext,
}

impl CxTestBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` on the app context, replacing any earlier value of
    /// the same type.
    #[must_use]
    pub fn app_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.app_context.insert(value);
        self
    }

    /// Registers `value` on the request context, replacing any earlier value
    /// of the same type.
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

/// Returns the [`Identity`] of `cx`'s scope.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call, or if an enclosing scope made
/// the identity ambiguous. Use [`try_identity`] when an identity may be
/// ambiguous.
#[must_use]
#[track_caller]
pub fn identity(cx: &Cx) -> Identity {
    match try_identity(cx) {
        Ok(identity) => identity,
        Err(error) => panic!("{error}"),
    }
}

/// Returns the [`Identity`] of `cx`'s scope, or an error if an enclosing
/// scope made it ambiguous.
///
/// # Errors
///
/// Returns an [`AmbiguousIdentityError`] naming the scope that introduced the
/// ambiguity.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call. To memoize a value that depends
/// on identity, read the identity before the call and pass it as an argument.
#[track_caller]
pub fn try_identity(cx: &Cx) -> Result<Identity, AmbiguousIdentityError> {
    assert!(
        cx.state.tracker.is_none(),
        "identity cannot be read inside memoized functions"
    );
    cx.identity.checked()
}

/// Returns the identity of `cx`'s scope without checking for ambiguity, for
/// deriving child identities from it.
#[doc(hidden)]
#[must_use]
pub fn identity_raw(cx: &Cx) -> Identity {
    cx.identity
}

/// Returns `cx` with its identity replaced by `identity`.
#[doc(hidden)]
#[must_use]
pub fn with_identity(cx: Cx, identity: Identity) -> Cx {
    Cx { identity, ..cx }
}

/// Returns the memoize cache of the request `cx` belongs to.
#[inline]
#[must_use]
#[doc(hidden)]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.state.shared.memoize_cache
}

/// Returns the abort store of the request `cx` belongs to.
#[inline]
#[must_use]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.state.shared.abort_store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Marker(u32);

    #[derive(Debug, PartialEq)]
    struct Other(&'static str);

    #[test]
    fn a_fresh_context_has_a_unique_id() {
        let first = Cx::new(Arc::new(AppContext::new()));
        let second = Cx::new(Arc::new(AppContext::new()));
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
        assert!(Arc::ptr_eq(&child.state, &cx.state));
        assert_eq!(child.id(), cx.id());
        assert!(std::ptr::eq(memoize_cache(&child), memoize_cache(&cx)));
        assert!(std::ptr::eq(
            request_context::<Marker>(&child),
            request_context::<Marker>(&cx)
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

        assert_eq!(try_request_context::<Marker>(&cx), None);
        assert_eq!(request_context::<Marker>(&child), &Marker(1));
    }

    #[test]
    fn with_shadows_without_touching_the_parent() {
        let cx = Cx::default().with(Marker(1));
        let child = cx.with(Marker(2));

        assert_eq!(request_context::<Marker>(&cx), &Marker(1));
        assert_eq!(request_context::<Marker>(&child), &Marker(2));
    }

    #[test]
    fn a_child_inherits_the_parent_context() {
        let cx = CxTestBuilder::new()
            .app_context(Other("app"))
            .request_context(Marker(7))
            .build();
        let child = cx.with(Other("request"));

        assert_eq!(request_context::<Marker>(&child), &Marker(7));
        assert_eq!(request_context::<Other>(&child), &Other("request"));
        assert_eq!(app_context::<Other>(&child), &Other("app"));
    }

    #[test]
    fn with_many_registers_every_value() {
        let cx = Cx::default().with_many((Marker(1), Other("many")));

        assert_eq!(request_context::<Marker>(&cx), &Marker(1));
        assert_eq!(request_context::<Other>(&cx), &Other("many"));
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
        let cx = CxTestBuilder::new().request_context(Marker(7)).build();
        let id = cx.id();
        let handle = cx.clone();
        drop(cx);

        assert_eq!(request_context::<Marker>(&handle).0, 7);
        assert_eq!(handle.id(), id);
    }

    #[test]
    fn handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Cx>();
    }
}
