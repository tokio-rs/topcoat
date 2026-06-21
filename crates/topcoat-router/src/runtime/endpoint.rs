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
}
