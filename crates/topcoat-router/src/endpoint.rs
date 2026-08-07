use std::{collections::HashMap, sync::Arc};

use http::Method;
use topcoat_core::context::{Cx, try_request_context};

use crate::{LayerId, Path, PathBuf, RawPathParamSpec};

/// The index of a registered route, with [`usize::MAX`] reserved to mean
/// "none".
///
/// This lets an [`Endpoint`] keep a dense `[RouteIndex; N]` table without the
/// padding an `[Option<usize>; N]` would carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteIndex(usize);

impl RouteIndex {
    /// The absence of a route.
    const NONE: Self = Self(usize::MAX);

    /// Wraps a route index.
    const fn new(index: usize) -> Self {
        debug_assert!(index != usize::MAX, "route index cannot be usize::MAX");
        Self(index)
    }

    /// Returns the wrapped index, or `None` if this is [`RouteIndex::NONE`].
    const fn get(self) -> Option<usize> {
        match self.0 {
            usize::MAX => None,
            index => Some(index),
        }
    }

    /// Returns `true` if no route is set.
    const fn is_none(self) -> bool {
        self.0 == usize::MAX
    }
}

impl Default for RouteIndex {
    fn default() -> Self {
        Self::NONE
    }
}

/// The standard HTTP methods, in the order their [`RouteIndex`] slots appear in
/// [`Endpoint::standard`]. Used only to name methods on the cold `Allow`-header
/// path; lookups go through [`standard_slot`].
const STANDARD_METHODS: [Method; 9] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::PATCH,
    Method::HEAD,
    Method::OPTIONS,
    Method::TRACE,
    Method::CONNECT,
];

const GET: usize = 0;
const HEAD: usize = 5;

/// Returns the [`Endpoint::standard`] slot for `method`, or `None` for an
/// extension method (which lives in [`Endpoint::other`] instead).
fn standard_slot(method: &Method) -> Option<usize> {
    match method.as_str() {
        "GET" => Some(GET),
        "POST" => Some(1),
        "PUT" => Some(2),
        "DELETE" => Some(3),
        "PATCH" => Some(4),
        "HEAD" => Some(HEAD),
        "OPTIONS" => Some(6),
        "TRACE" => Some(7),
        "CONNECT" => Some(8),
        _ => None,
    }
}

/// The set of routes registered at a single path, indexed by HTTP method.
///
/// The standard methods occupy a fixed-size array for O(1), allocation-free
/// lookup; the rare custom methods spill into a map that is usually empty.
#[derive(Debug, Default)]
pub(crate) struct Endpoint {
    standard: [RouteIndex; STANDARD_METHODS.len()],
    other: HashMap<Method, usize>,
    /// The route handling every method without a registration of its own.
    /// Routes registered for a specific method take precedence.
    any: RouteIndex,
    /// The URL path this endpoint serves, shared with every request matched to
    /// it. Group segments are stripped, since they are not part of the URL and
    /// routes that differ only in them land on one endpoint.
    path: Arc<PathBuf>,
    /// Interned path parameter names and capture kinds for this endpoint.
    path_params: Box<[RawPathParamSpec]>,
    /// The layers wrapping every route at this path, as ids into the router's
    /// layer table, precomputed at build time and ordered from least- to
    /// most-specific so the outermost layer runs first. Shared by every method
    /// at the path, including the `405` fallback.
    layers: Box<[LayerId]>,
}

impl Endpoint {
    pub(crate) fn new(
        path: Arc<PathBuf>,
        path_params: Box<[RawPathParamSpec]>,
        layers: Box<[LayerId]>,
    ) -> Self {
        Self {
            standard: Default::default(),
            other: HashMap::new(),
            any: RouteIndex::NONE,
            path,
            path_params,
            layers,
        }
    }

    /// Returns the URL path this endpoint serves, ready to be cloned onto a
    /// matched request's context.
    pub(crate) fn path(&self) -> &Arc<PathBuf> {
        &self.path
    }

    /// Returns the route index registered specifically for `method`, if any.
    /// The any-method route is not consulted; read it with [`any`](Self::any).
    pub(crate) fn get(&self, method: &Method) -> Option<usize> {
        match standard_slot(method) {
            Some(slot) => self.standard[slot].get(),
            None => self.other.get(method).copied(),
        }
    }

    /// Returns the route index handling every unregistered method, if one is
    /// set.
    pub(crate) fn any(&self) -> Option<usize> {
        self.any.get()
    }

    /// Registers `index` as the route handling `method`.
    pub(crate) fn insert(&mut self, method: Method, index: usize) {
        match standard_slot(&method) {
            Some(slot) => self.standard[slot] = RouteIndex::new(index),
            None => {
                self.other.insert(method, index);
            }
        }
    }

    /// Registers `index` as the route handling every method that has no
    /// registration of its own.
    pub(crate) fn insert_any(&mut self, index: usize) {
        self.any = RouteIndex::new(index);
    }

    /// Points the `HEAD` slot at the `GET` route unless a `HEAD` route was
    /// registered explicitly, so `HEAD` requests reuse the `GET` handler.
    pub(crate) fn alias_head_to_get(&mut self) {
        if self.standard[HEAD].is_none() {
            self.standard[HEAD] = self.standard[GET];
        }
    }

    /// Iterates over the methods this path supports.
    pub(crate) fn methods(&self) -> impl Iterator<Item = &Method> {
        STANDARD_METHODS
            .iter()
            .enumerate()
            .filter(|(slot, _)| !self.standard[*slot].is_none())
            .map(|(_, method)| method)
            .chain(self.other.keys())
    }

    /// Returns the path parameter names and capture kinds for this endpoint.
    pub(crate) fn path_params(&self) -> &[RawPathParamSpec] {
        &self.path_params
    }

    /// Returns the precomputed layer stack wrapping this path's routes, as ids
    /// into the router's layer table.
    pub(crate) fn layers(&self) -> &[LayerId] {
        &self.layers
    }
}

/// The path of the endpoint a request matched, held on its request context.
///
/// The router stores one value per endpoint and hands each matched request a
/// clone of it, so reading the path allocates nothing. Read it with
/// [`endpoint_path`].
#[derive(Debug, Clone)]
pub(crate) struct EndpointPath(pub(crate) Arc<PathBuf>);

/// Returns the path of the endpoint the current request matched.
///
/// This is the path pattern the endpoint was registered under rather than the
/// URL that was requested, so parameters keep their `{name}` form. Group
/// segments are not part of it: they bind layouts and layers at build time and
/// never reach the URL. Read the requested URL with
/// [`uri`](crate::request::uri) instead.
///
/// # Panics
///
/// Panics if the request matched no endpoint.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::endpoint_path};
///
/// fn log_line(cx: &Cx) -> String {
///     // The pattern, not the URL: `/users/{id}` for a request to `/users/42`.
///     format!("handled {}", endpoint_path(cx))
/// }
/// ```
#[must_use]
#[track_caller]
pub fn endpoint_path(cx: &Cx) -> &Path {
    match try_request_context::<EndpointPath>(cx) {
        Some(EndpointPath(path)) => path,
        None => panic!("this request matched no endpoint, so it has no endpoint path"),
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    // -- RouteIndex --

    #[test]
    fn route_index_wraps_and_unwraps() {
        let index = RouteIndex::new(7);
        assert_eq!(index.get(), Some(7));
        assert!(!index.is_none());
    }

    #[test]
    fn route_index_zero_is_a_real_index() {
        // Index 0 must be distinguishable from "none".
        let index = RouteIndex::new(0);
        assert_eq!(index.get(), Some(0));
        assert!(!index.is_none());
    }

    #[test]
    fn route_index_none_is_absent() {
        assert_eq!(RouteIndex::NONE.get(), None);
        assert!(RouteIndex::NONE.is_none());
        assert_eq!(RouteIndex::default(), RouteIndex::NONE);
    }

    // -- Endpoint: standard methods --

    #[test]
    fn empty_endpoint_has_no_routes() {
        let endpoint = Endpoint::default();
        assert_eq!(endpoint.get(&Method::GET), None);
        assert_eq!(endpoint.get(&Method::POST), None);
        assert_eq!(endpoint.methods().count(), 0);
    }

    #[test]
    fn inserts_and_reads_back_standard_methods() {
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::GET, 0);
        endpoint.insert(Method::POST, 1);
        endpoint.insert(Method::DELETE, 2);

        assert_eq!(endpoint.get(&Method::GET), Some(0));
        assert_eq!(endpoint.get(&Method::POST), Some(1));
        assert_eq!(endpoint.get(&Method::DELETE), Some(2));
        // A method that was never registered is still absent.
        assert_eq!(endpoint.get(&Method::PUT), None);
    }

    #[test]
    fn insert_overwrites_the_same_method() {
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::GET, 0);
        endpoint.insert(Method::GET, 5);
        assert_eq!(endpoint.get(&Method::GET), Some(5));
    }

    // -- Endpoint: extension methods --

    #[test]
    fn inserts_and_reads_back_extension_methods() {
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let mut endpoint = Endpoint::default();
        endpoint.insert(purge.clone(), 3);

        assert_eq!(endpoint.get(&purge), Some(3));
        assert_eq!(endpoint.get(&Method::GET), None);
    }

    // -- Endpoint: any-method route --

    #[test]
    fn any_is_absent_by_default() {
        let endpoint = Endpoint::default();
        assert_eq!(endpoint.any(), None);
    }

    #[test]
    fn insert_any_does_not_affect_per_method_lookups() {
        let mut endpoint = Endpoint::default();
        endpoint.insert_any(7);

        assert_eq!(endpoint.any(), Some(7));
        // `get` and the `Allow`-header iterator only cover per-method
        // registrations.
        assert_eq!(endpoint.get(&Method::GET), None);
        assert_eq!(endpoint.methods().count(), 0);
    }

    // -- Endpoint: HEAD aliasing --

    #[test]
    fn alias_points_head_at_get() {
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::GET, 4);
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), Some(4));
    }

    #[test]
    fn alias_does_not_override_explicit_head() {
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::GET, 4);
        endpoint.insert(Method::HEAD, 9);
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), Some(9));
    }

    #[test]
    fn alias_without_get_leaves_head_absent() {
        let mut endpoint = Endpoint::default();
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), None);
    }

    // -- Endpoint: methods iterator --

    #[test]
    fn methods_lists_standard_then_extension() {
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::POST, 1);
        endpoint.insert(Method::GET, 0);
        endpoint.insert(purge.clone(), 2);

        let methods: Vec<&Method> = endpoint.methods().collect();
        // Standard methods come first, in `STANDARD_METHODS` order, regardless of
        // insertion order; extension methods follow.
        assert_eq!(methods, vec![&Method::GET, &Method::POST, &purge]);
    }

    // -- endpoint_path --

    #[test]
    fn reads_the_matched_endpoint_path() {
        let path = Arc::new(Path::new("/users/{id}").to_owned());
        let cx = CxTestBuilder::new()
            .request_context(EndpointPath(path))
            .build();

        assert_eq!(endpoint_path(&cx), Path::new("/users/{id}"));
    }

    #[test]
    #[should_panic(expected = "matched no endpoint")]
    fn endpoint_path_panics_without_a_match() {
        let _ = endpoint_path(&Cx::default());
    }
}
