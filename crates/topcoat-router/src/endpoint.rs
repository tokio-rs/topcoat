use std::{
    borrow::Cow,
    collections::HashMap,
    ops::{Index, IndexMut},
};

use http::Method;

use crate::{LayerIndex, Path, RouteIndex};

/// The standard HTTP methods, in the order their route slots appear in
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

/// The set of routes registered at a single URL path, indexed by HTTP method.
///
/// The router matches a request URL to one endpoint, then picks the route
/// registered for the request's method. Read the endpoint a request matched
/// with [`endpoint`](crate::endpoint).
///
/// The standard methods occupy a fixed-size array for O(1), allocation-free
/// lookup; the rare custom methods spill into a map that is usually empty.
#[derive(Debug)]
pub struct Endpoint {
    standard: [Option<RouteIndex>; STANDARD_METHODS.len()],
    other: HashMap<Method, RouteIndex>,
    /// The route handling every method without a registration of its own.
    /// Routes registered for a specific method take precedence.
    any: Option<RouteIndex>,
    /// The URL path this endpoint serves. Group segments are stripped, since
    /// they are not part of the URL and routes that differ only in them land
    /// on one endpoint.
    path: Box<str>,
    /// The layers wrapping every route at this path, as indices into the
    /// router's layer table, precomputed at build time and ordered from least-
    /// to most-specific so the outermost layer runs first. Shared by every
    /// method at the path, including the `405` fallback.
    layers: Box<[LayerIndex]>,
}

impl Endpoint {
    pub(crate) fn new(path: &Path, layers: Box<[LayerIndex]>) -> Self {
        Self {
            standard: [None; STANDARD_METHODS.len()],
            other: HashMap::new(),
            any: None,
            path: path.as_str().into(),
            layers,
        }
    }

    /// Returns the URL path this endpoint serves.
    ///
    /// This is the path pattern the endpoint was registered under rather than
    /// a requested URL, so parameters keep their `{name}` form. Group
    /// segments are not part of it: they bind layouts and layers at build
    /// time and never reach the URL.
    #[must_use]
    pub fn path(&self) -> &Path {
        Path::new_unchecked(&self.path)
    }

    /// Returns the route registered specifically for `method`, if any. The
    /// any-method route is not consulted; read it with [`any`](Self::any).
    pub(crate) fn get(&self, method: &Method) -> Option<RouteIndex> {
        match standard_slot(method) {
            Some(slot) => self.standard[slot],
            None => self.other.get(method).copied(),
        }
    }

    /// Returns the route handling every unregistered method, if one is set.
    pub(crate) fn any(&self) -> Option<RouteIndex> {
        self.any
    }

    /// Registers `index` as the route handling `method`.
    pub(crate) fn insert(&mut self, method: Method, index: RouteIndex) {
        match standard_slot(&method) {
            Some(slot) => self.standard[slot] = Some(index),
            None => {
                self.other.insert(method, index);
            }
        }
    }

    /// Registers `index` as the route handling every method that has no
    /// registration of its own.
    pub(crate) fn insert_any(&mut self, index: RouteIndex) {
        self.any = Some(index);
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
            .filter(|(slot, _)| self.standard[*slot].is_some())
            .map(|(_, method)| method)
            .chain(self.other.keys())
    }

    /// Returns the precomputed layer stack wrapping this path's routes, as
    /// indices into the router's layer table.
    pub(crate) fn layers(&self) -> &[LayerIndex] {
        &self.layers
    }
}

/// The position of an [`Endpoint`] in a router's [`Endpoints`] table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndpointIndex(usize);

/// The endpoints registered on a router, together with the matcher that
/// resolves a request URL to one of them.
///
/// Endpoints are [`push`](Self::push)ed as the router is built, then only
/// queried: [`at`](Self::at) matches a request URL, and indexing by
/// [`EndpointIndex`] resolves a stored index back to its endpoint.
#[derive(Default)]
pub(crate) struct Endpoints {
    endpoints: Vec<Endpoint>,
    matcher: matchit::Router<EndpointIndex>,
}

impl Endpoints {
    /// Registers `endpoint` under the matcher path `path`, returning the
    /// [`EndpointIndex`] that now identifies it.
    ///
    /// # Panics
    ///
    /// Panics if `path` conflicts with an already registered one.
    #[track_caller]
    pub(crate) fn push(&mut self, path: Cow<'static, str>, endpoint: Endpoint) -> EndpointIndex {
        let index = EndpointIndex(self.endpoints.len());
        self.matcher
            .insert(path.clone(), index)
            .unwrap_or_else(|error| panic!("failed to register route {path:?}: {error}"));
        self.endpoints.push(endpoint);
        index
    }

    /// Matches a request URL against the registered endpoints, returning the
    /// matched endpoint and the path parameters the URL captured.
    pub(crate) fn at<'s, 'url>(
        &'s self,
        url: &'url str,
    ) -> Option<(EndpointIndex, &'s Endpoint, matchit::Params<'s, 'url>)> {
        let matched = self.matcher.at(url).ok()?;
        let index = *matched.value;
        Some((index, &self.endpoints[index.0], matched.params))
    }
}

impl Index<EndpointIndex> for Endpoints {
    type Output = Endpoint;

    fn index(&self, EndpointIndex(index): EndpointIndex) -> &Self::Output {
        &self.endpoints[index]
    }
}

impl IndexMut<EndpointIndex> for Endpoints {
    fn index_mut(&mut self, EndpointIndex(index): EndpointIndex) -> &mut Self::Output {
        &mut self.endpoints[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(index: usize) -> RouteIndex {
        RouteIndex::new(index)
    }

    /// An endpoint with no routes registered, at a path no test cares about.
    fn empty() -> Endpoint {
        Endpoint::new(Path::new("/x"), Box::new([]))
    }

    // -- Endpoint: standard methods --

    #[test]
    fn empty_endpoint_has_no_routes() {
        let endpoint = empty();
        assert_eq!(endpoint.get(&Method::GET), None);
        assert_eq!(endpoint.get(&Method::POST), None);
        assert_eq!(endpoint.methods().count(), 0);
    }

    #[test]
    fn inserts_and_reads_back_standard_methods() {
        let mut endpoint = empty();
        endpoint.insert(Method::GET, route(0));
        endpoint.insert(Method::POST, route(1));
        endpoint.insert(Method::DELETE, route(2));

        assert_eq!(endpoint.get(&Method::GET), Some(route(0)));
        assert_eq!(endpoint.get(&Method::POST), Some(route(1)));
        assert_eq!(endpoint.get(&Method::DELETE), Some(route(2)));
        // A method that was never registered is still absent.
        assert_eq!(endpoint.get(&Method::PUT), None);
    }

    #[test]
    fn insert_overwrites_the_same_method() {
        let mut endpoint = empty();
        endpoint.insert(Method::GET, route(0));
        endpoint.insert(Method::GET, route(5));
        assert_eq!(endpoint.get(&Method::GET), Some(route(5)));
    }

    // -- Endpoint: extension methods --

    #[test]
    fn inserts_and_reads_back_extension_methods() {
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let mut endpoint = empty();
        endpoint.insert(purge.clone(), route(3));

        assert_eq!(endpoint.get(&purge), Some(route(3)));
        assert_eq!(endpoint.get(&Method::GET), None);
    }

    // -- Endpoint: any-method route --

    #[test]
    fn any_is_absent_by_default() {
        let endpoint = empty();
        assert_eq!(endpoint.any(), None);
    }

    #[test]
    fn insert_any_does_not_affect_per_method_lookups() {
        let mut endpoint = empty();
        endpoint.insert_any(route(7));

        assert_eq!(endpoint.any(), Some(route(7)));
        // `get` and the `Allow`-header iterator only cover per-method
        // registrations.
        assert_eq!(endpoint.get(&Method::GET), None);
        assert_eq!(endpoint.methods().count(), 0);
    }

    // -- Endpoint: HEAD aliasing --

    #[test]
    fn alias_points_head_at_get() {
        let mut endpoint = empty();
        endpoint.insert(Method::GET, route(4));
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), Some(route(4)));
    }

    #[test]
    fn alias_does_not_override_explicit_head() {
        let mut endpoint = empty();
        endpoint.insert(Method::GET, route(4));
        endpoint.insert(Method::HEAD, route(9));
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), Some(route(9)));
    }

    #[test]
    fn alias_without_get_leaves_head_absent() {
        let mut endpoint = empty();
        endpoint.alias_head_to_get();
        assert_eq!(endpoint.get(&Method::HEAD), None);
    }

    // -- Endpoint: methods iterator --

    #[test]
    fn methods_lists_standard_then_extension() {
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let mut endpoint = empty();
        endpoint.insert(Method::POST, route(1));
        endpoint.insert(Method::GET, route(0));
        endpoint.insert(purge.clone(), route(2));

        let methods: Vec<&Method> = endpoint.methods().collect();
        // Standard methods come first, in `STANDARD_METHODS` order, regardless of
        // insertion order; extension methods follow.
        assert_eq!(methods, vec![&Method::GET, &Method::POST, &purge]);
    }

    // -- Endpoints --

    fn endpoint_at(path: &'static str) -> (Cow<'static, str>, Endpoint) {
        let path = Path::new(path);
        (path.to_matchit_path(), Endpoint::new(path, Box::new([])))
    }

    #[test]
    fn push_assigns_sequential_indices() {
        let mut endpoints = Endpoints::default();
        let (path_x, x) = endpoint_at("/x");
        let (path_y, y) = endpoint_at("/y");
        assert_eq!(endpoints.push(path_x, x), EndpointIndex(0));
        assert_eq!(endpoints.push(path_y, y), EndpointIndex(1));
    }

    #[test]
    fn at_matches_a_url_to_its_endpoint() {
        let mut endpoints = Endpoints::default();
        let (path, endpoint) = endpoint_at("/users/{id}");
        let pushed = endpoints.push(path, endpoint);

        let (index, endpoint, params) = endpoints.at("/users/42").unwrap();
        assert_eq!(index, pushed);
        assert_eq!(endpoint.path(), Path::new("/users/{id}"));
        assert_eq!(params.get("id"), Some("42"));
        assert_eq!(endpoints[index].path(), Path::new("/users/{id}"));
    }

    #[test]
    fn at_returns_none_for_an_unmatched_url() {
        let mut endpoints = Endpoints::default();
        let (path, endpoint) = endpoint_at("/x");
        endpoints.push(path, endpoint);
        assert!(endpoints.at("/missing").is_none());
    }

    #[test]
    #[should_panic(expected = "failed to register route")]
    fn push_rejects_conflicting_paths() {
        let mut endpoints = Endpoints::default();
        let (path, endpoint) = endpoint_at("/users/{id}");
        endpoints.push(path, endpoint);
        let (path, endpoint) = endpoint_at("/users/{user_id}");
        endpoints.push(path, endpoint);
    }
}
