mod app_context;
mod id;
mod request_context;
mod tracking;

use std::{any::Any, sync::Arc};

pub use app_context::*;
pub use id::*;
pub use request_context::*;
pub(crate) use tracking::*;

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
/// [`with`](Self::with) and [`with_many`](Self::with_many) derive a child
/// handle whose request context holds additional values, leaving the parent
/// untouched. Cloning a handle is cheap; work that outlives the handler, such
/// as a streaming response body or a WebSocket task, should move an owned clone into
/// the work.
#[derive(Debug, Default, Clone)]
pub struct Cx {
    /// The state shared by every handle serving this request.
    shared: Arc<RequestShared>,
    /// The request context visible to this handle's scope.
    request_context: Arc<RequestContext>,
    /// The tracker recording this handle's request context reads, if any.
    tracker: Option<Arc<ContextTracker>>,
}

impl Cx {
    /// Creates the context for one request over the shared app context, with an
    /// empty request context.
    #[must_use]
    pub fn new(app_context: Arc<AppContext>) -> Self {
        Self::from_parts(app_context, RequestContext::new())
    }

    /// Creates a `Cx` from the given app and request contexts.
    fn from_parts(app_context: Arc<AppContext>, request_context: RequestContext) -> Self {
        Self {
            shared: Arc::new(RequestShared {
                id: CxId::new(),
                app_context,
                memoize_cache: MemoizeCache::new(),
                abort_store: AbortStore::new(),
            }),
            request_context: Arc::new(request_context),
            tracker: None,
        }
    }

    /// Returns this request's unique [`CxId`].
    #[inline]
    #[must_use]
    pub fn id(&self) -> CxId {
        self.shared.id
    }

    /// Returns a child handle whose request context also holds `value`.
    ///
    /// The child inherits every other request context value and shares the
    /// rest of the request state, such as the app context and the memoize
    /// cache, with `self`. Registering a type that is already present shadows
    /// the inherited value: lookups through the child see `value`, while
    /// lookups through `self` still see the original.
    #[must_use]
    pub fn with<T>(&self, value: T) -> Cx
    where
        T: Any + Send + Sync,
    {
        let mut request_context = (*self.request_context).clone();
        request_context.insert(value);
        self.scope(request_context)
    }

    /// Returns a child handle whose request context also holds every value in
    /// `values`, a tuple of context values.
    ///
    /// Behaves like chained [`with`](Self::with) calls, but builds the child's
    /// request context in one step.
    #[must_use]
    pub fn with_many<V>(&self, values: V) -> Cx
    where
        V: ContextValues,
    {
        let mut request_context = (*self.request_context).clone();
        values.install(&mut request_context);
        self.scope(request_context)
    }

    /// Wraps a derived request context into a child handle sharing this
    /// request's state.
    fn scope(&self, request_context: RequestContext) -> Cx {
        Cx {
            shared: Arc::clone(&self.shared),
            request_context: Arc::new(request_context),
            tracker: self.tracker.clone(),
        }
    }

    /// Returns a child handle whose request context reads are recorded, along
    /// with the tracker collecting them.
    ///
    /// The child shares this handle's scope, and that scope is also the
    /// tracker's entry scope. A tracker inherited from an enclosing `track`
    /// call is replaced, not stacked: reads made through the child and its
    /// descendants are recorded by the new tracker only.
    // TODO: unused only until the memoize integration lands; remove with it.
    #[allow(dead_code)]
    pub(crate) fn track(&self) -> (Cx, Arc<ContextTracker>) {
        let tracker = Arc::new(ContextTracker::new(Arc::clone(&self.request_context)));
        let child = Cx {
            shared: Arc::clone(&self.shared),
            request_context: Arc::clone(&self.request_context),
            tracker: Some(Arc::clone(&tracker)),
        };
        (child, tracker)
    }
}

/// The state shared by every handle to one request's [`Cx`].
#[derive(Debug, Default)]
struct RequestShared {
    id: CxId,
    app_context: Arc<AppContext>,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

/// Assembles a [`Cx`] from scratch, for tests.
///
/// Unlike [`Cx::new`], which only takes an existing shared app context,
/// `CxTestBuilder` populates both app and request context.
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
    &cx.shared.memoize_cache
}

#[inline]
#[must_use]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.shared.abort_store
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
