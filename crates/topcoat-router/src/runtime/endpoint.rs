use std::collections::HashMap;

use http::Method;

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
    /// The layers wrapping every route at this path, as indices into the
    /// router's layer list, precomputed at build time and ordered from
    /// least- to most-specific so the outermost layer runs first. Shared by
    /// every method at the path, including the `405` fallback.
    layers: Box<[usize]>,
}

impl Endpoint {
    /// Returns the route index handling `method`, if any.
    pub(crate) fn get(&self, method: &Method) -> Option<usize> {
        match standard_slot(method) {
            Some(slot) => self.standard[slot].get(),
            None => self.other.get(method).copied(),
        }
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

    /// Records the precomputed layer stack wrapping this path's routes.
    pub(crate) fn set_layers(&mut self, layers: Box<[usize]>) {
        self.layers = layers;
    }

    /// Returns the precomputed layer stack wrapping this path's routes, as
    /// indices into the router's layer list.
    pub(crate) fn layers(&self) -> &[usize] {
        &self.layers
    }
}

#[cfg(test)]
mod tests {
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
}
