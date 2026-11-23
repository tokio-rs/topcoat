use std::fmt;

use bumpalo::Bump;
use http::Method;

use crate::{Path, RouteEntry};

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
/// An endpoint borrows from the router it belongs to.
#[derive(Clone, Copy)]
pub struct Endpoint<'r> {
    /// The URL path this endpoint serves. Group segments are stripped, since
    /// they are not part of the URL and routes that differ only in them land
    /// on one endpoint.
    path: &'r Path,
    standard: [Option<&'r RouteEntry<'r>>; STANDARD_METHODS.len()],
    /// The route handling every method without a registration of its own.
    /// Routes registered for a specific method take precedence.
    any: Option<&'r RouteEntry<'r>>,
    /// The routes registered for extension methods. Almost every endpoint
    /// has none.
    other: &'r [(&'r Method, &'r RouteEntry<'r>)],
}

impl<'r> Endpoint<'r> {
    /// Creates an endpoint serving `path` with no routes.
    pub(crate) fn new(path: &'r Path) -> Self {
        Self {
            path,
            standard: [None; STANDARD_METHODS.len()],
            any: None,
            other: &[],
        }
    }

    /// Returns a copy of this endpoint's method table serving `path` instead.
    pub(crate) fn with_path(&self, path: &'r Path) -> Self {
        Self { path, ..*self }
    }

    /// Returns the URL path this endpoint serves.
    ///
    /// This is the path pattern the endpoint was registered under rather than
    /// a requested URL, so parameters keep their `{name}` form. Group
    /// segments are not part of it: they bind layouts and layers at build
    /// time and never reach the URL.
    #[must_use]
    pub fn path(&self) -> &'r Path {
        self.path
    }

    /// Returns the route registered specifically for `method`, if any. The
    /// any-method route is not consulted; read it with [`any`](Self::any).
    pub(crate) fn get(&self, method: &Method) -> Option<&'r RouteEntry<'r>> {
        match standard_slot(method) {
            Some(slot) => self.standard[slot],
            None => self
                .other
                .iter()
                .find(|(registered, _)| *registered == method)
                .map(|&(_, entry)| entry),
        }
    }

    /// Returns the route handling every unregistered method, if one is set.
    pub(crate) fn any(&self) -> Option<&'r RouteEntry<'r>> {
        self.any
    }

    /// Registers `entry` as the route handling `method`.
    ///
    /// The list of extension methods lives in `arena`.
    pub(crate) fn insert(
        &mut self,
        arena: &'r Bump,
        method: &'r Method,
        entry: &'r RouteEntry<'r>,
    ) {
        if let Some(slot) = standard_slot(method) {
            self.standard[slot] = Some(entry);
        } else {
            let mut other: Vec<_> = self
                .other
                .iter()
                .copied()
                .filter(|(registered, _)| *registered != method)
                .collect();
            other.push((method, entry));
            self.other = arena.alloc_slice_copy(&other);
        }
    }

    /// Registers `entry` as the route handling every method that has no
    /// registration of its own.
    pub(crate) fn insert_any(&mut self, entry: &'r RouteEntry<'r>) {
        self.any = Some(entry);
    }

    /// Points the `HEAD` slot at the `GET` route unless a `HEAD` route was
    /// registered explicitly, so `HEAD` requests reuse the `GET` handler.
    pub(crate) fn alias_head_to_get(&mut self) {
        if self.standard[HEAD].is_none() {
            self.standard[HEAD] = self.standard[GET];
        }
    }

    /// Iterates over the methods with a route registered at this endpoint,
    /// standard methods first.
    pub fn methods(&self) -> impl Iterator<Item = &Method> {
        STANDARD_METHODS
            .iter()
            .enumerate()
            .filter(|(slot, _)| self.standard[*slot].is_some())
            .map(|(_, method)| method)
            .chain(self.other.iter().map(|&(method, _)| method))
    }
}

impl fmt::Debug for Endpoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("path", &self.path)
            .field("methods", &self.methods().collect::<Vec<_>>())
            .field("any", &self.any.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use topcoat_core::context::Cx;

    use super::*;
    use crate::{Body, Handler, RouteFn, RouteFuture};

    fn unused(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        unreachable!()
    }

    fn route() -> RouteFn {
        RouteFn::new(Method::GET, "/x", unused)
    }

    /// A route entry for `route`. Tests tell entries apart by their address.
    fn entry(route: &RouteFn) -> RouteEntry<'_> {
        RouteEntry {
            handler: Handler::Route(route),
            layers: &[],
            context: None,
        }
    }

    /// Returns whether `found` is the entry `expected`, not just one equal
    /// to it.
    fn is(found: Option<&RouteEntry<'_>>, expected: &RouteEntry<'_>) -> bool {
        found.is_some_and(|found| ptr::eq(found, expected))
    }

    /// An endpoint with no routes registered, at a path no test cares about.
    fn empty<'r>() -> Endpoint<'r> {
        Endpoint::new(Path::new("/x"))
    }

    // -- Standard methods --

    #[test]
    fn empty_endpoint_has_no_routes() {
        let endpoint = empty();
        assert!(endpoint.get(&Method::GET).is_none());
        assert!(endpoint.get(&Method::POST).is_none());
        assert_eq!(endpoint.methods().count(), 0);
    }

    #[test]
    fn inserts_and_reads_back_standard_methods() {
        let arena = Bump::new();
        let route = route();
        let (get, post, delete) = (entry(&route), entry(&route), entry(&route));
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::GET, &get);
        endpoint.insert(&arena, &Method::POST, &post);
        endpoint.insert(&arena, &Method::DELETE, &delete);

        assert!(is(endpoint.get(&Method::GET), &get));
        assert!(is(endpoint.get(&Method::POST), &post));
        assert!(is(endpoint.get(&Method::DELETE), &delete));
        // A method that was never registered is still absent.
        assert!(endpoint.get(&Method::PUT).is_none());
    }

    #[test]
    fn insert_overwrites_the_same_method() {
        let arena = Bump::new();
        let route = route();
        let (first, second) = (entry(&route), entry(&route));
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::GET, &first);
        endpoint.insert(&arena, &Method::GET, &second);
        assert!(is(endpoint.get(&Method::GET), &second));
    }

    // -- Extension methods --

    #[test]
    fn inserts_and_reads_back_extension_methods() {
        let arena = Bump::new();
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let route = route();
        let purged = entry(&route);
        let mut endpoint = empty();
        endpoint.insert(&arena, &purge, &purged);

        assert!(is(endpoint.get(&purge), &purged));
        assert!(endpoint.get(&Method::GET).is_none());
    }

    #[test]
    fn insert_overwrites_the_same_extension_method() {
        let arena = Bump::new();
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let route = route();
        let (first, second) = (entry(&route), entry(&route));
        let mut endpoint = empty();
        endpoint.insert(&arena, &purge, &first);
        endpoint.insert(&arena, &purge, &second);

        assert!(is(endpoint.get(&purge), &second));
        // The method is still listed once.
        assert_eq!(endpoint.methods().count(), 1);
    }

    // -- Any-method route --

    #[test]
    fn any_is_absent_by_default() {
        let endpoint = empty();
        assert!(endpoint.any().is_none());
    }

    #[test]
    fn insert_any_does_not_affect_per_method_lookups() {
        let route = route();
        let any = entry(&route);
        let mut endpoint = empty();
        endpoint.insert_any(&any);

        assert!(is(endpoint.any(), &any));
        // `get` and the `Allow`-header iterator only cover per-method
        // registrations.
        assert!(endpoint.get(&Method::GET).is_none());
        assert_eq!(endpoint.methods().count(), 0);
    }

    // -- HEAD aliasing --

    #[test]
    fn alias_points_head_at_get() {
        let arena = Bump::new();
        let route = route();
        let get = entry(&route);
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::GET, &get);
        endpoint.alias_head_to_get();
        assert!(is(endpoint.get(&Method::HEAD), &get));
    }

    #[test]
    fn alias_does_not_override_explicit_head() {
        let arena = Bump::new();
        let route = route();
        let (get, head) = (entry(&route), entry(&route));
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::GET, &get);
        endpoint.insert(&arena, &Method::HEAD, &head);
        endpoint.alias_head_to_get();
        assert!(is(endpoint.get(&Method::HEAD), &head));
    }

    #[test]
    fn alias_without_get_leaves_head_absent() {
        let mut endpoint = empty();
        endpoint.alias_head_to_get();
        assert!(endpoint.get(&Method::HEAD).is_none());
    }

    // -- Methods iterator --

    #[test]
    fn methods_lists_standard_then_extension() {
        let arena = Bump::new();
        let purge = Method::from_bytes(b"PURGE").unwrap();
        let route = route();
        let entry = entry(&route);
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::POST, &entry);
        endpoint.insert(&arena, &Method::GET, &entry);
        endpoint.insert(&arena, &purge, &entry);

        let methods: Vec<&Method> = endpoint.methods().collect();
        // Standard methods come first, in `STANDARD_METHODS` order, regardless of
        // insertion order; extension methods follow.
        assert_eq!(methods, vec![&Method::GET, &Method::POST, &purge]);
    }

    // -- Serving another path --

    #[test]
    fn with_path_keeps_the_routes_and_changes_the_path() {
        let arena = Bump::new();
        let route = route();
        let get = entry(&route);
        let mut endpoint = empty();
        endpoint.insert(&arena, &Method::GET, &get);

        let twin = endpoint.with_path(Path::new("/x/"));
        assert_eq!(twin.path(), Path::new("/x/"));
        assert!(is(twin.get(&Method::GET), &get));
    }
}
