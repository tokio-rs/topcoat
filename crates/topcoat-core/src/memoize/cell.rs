use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::{Mutex, PoisonError},
};

use elsa::sync::FrozenVec;

use super::recursion;
use crate::context::{ContextRead, Cx, RequestContext};

/// One cached result together with the request context reads that produced
/// it.
///
/// A variant is reusable for a caller whose scope still resolves every
/// recorded read to the binding that was observed. A body that read no
/// request context has an empty read list, so its variant is reusable from
/// every scope.
struct Variant<V> {
    value: V,
    reads: Vec<ContextRead>,
}

impl<V> Variant<V> {
    /// Returns whether the scope `context` still resolves every recorded
    /// read to the binding that was observed.
    fn matches(&self, context: &RequestContext) -> bool {
        self.reads.iter().all(|read| read.matches(context))
    }

    /// Hands out the cached value, replaying the variant's reads into the
    /// tracker of an enclosing tracked call so that nested memoized calls
    /// propagate their dependencies.
    fn reuse(&self, cx: &Cx) -> &V {
        if let Some(outer) = cx.tracker() {
            outer.merge(&self.reads);
        }
        &self.value
    }
}

/// The variants computed for one `(function, arguments)` entry, one per set
/// of context bindings the body was observed under.
struct Variants<V> {
    /// Variants are boxed so their addresses stay stable while the list
    /// grows, letting the cell hand out `&V` references tied to the cache.
    entries: FrozenVec<Box<Variant<V>>>,
}

impl<V> Variants<V> {
    /// Returns the value of the first variant reusable from `cx`'s scope.
    fn reuse(&self, cx: &Cx) -> Option<&V> {
        self.entries
            .iter()
            .find(|variant| variant.matches(cx.request_context()))
            .map(|variant| variant.reuse(cx))
    }

    /// Stores a computed value together with the reads observed while
    /// computing it, handing the value back out.
    fn insert(&self, cx: &Cx, value: V, reads: Vec<ContextRead>) -> &V {
        self.entries
            .push_get(Box::new(Variant { value, reads }))
            .reuse(cx)
    }
}

// Not derived: the derive would needlessly bound `V: Default`.
impl<V> Default for Variants<V> {
    fn default() -> Self {
        Self {
            entries: FrozenVec::new(),
        }
    }
}

/// The cell behind one synchronous `(function, arguments)` entry.
pub(super) struct SyncMemoizeCell<V> {
    variants: Variants<V>,
    /// Serializes misses so concurrent callers with the same scope run the
    /// body once. The gate guards no data, so a poisoned lock (a panicked
    /// body) is safe to keep using.
    gate: Mutex<()>,
    recursion: recursion::Guard,
}

impl<V> SyncMemoizeCell<V> {
    /// Returns the value of the variant reusable from `cx`'s scope, or `None`
    /// if the body has not run under bindings that scope still agrees with.
    pub(super) fn reuse(&self, cx: &Cx) -> Option<&V> {
        self.variants.reuse(cx)
    }

    /// Returns the value reusable from `cx`'s scope, running `initialize`
    /// under a fresh tracker to compute a variant when there is none.
    ///
    /// `Marker` is the memoized function's type and only names it in the
    /// recursion panic.
    pub(super) fn get_or_init<Marker, I>(&self, cx: &Cx, initialize: I) -> &V
    where
        I: FnOnce(&Cx) -> V,
    {
        if let Some(value) = self.variants.reuse(cx) {
            return value;
        }
        // Asserted before taking the gate so reentry panics instead of deadlocking on it.
        self.recursion.assert_not_recursive::<Marker>();
        let _gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(value) = self.variants.reuse(cx) {
            return value;
        }
        let (child, tracker) = cx.track();
        let value = self.recursion.scope(|| initialize(&child));
        self.variants.insert(cx, value, tracker.reads())
    }
}

// Not derived: the derive would needlessly bound `V: Default`.
impl<V> Default for SyncMemoizeCell<V> {
    fn default() -> Self {
        Self {
            variants: Variants::default(),
            gate: Mutex::new(()),
            recursion: recursion::Guard::default(),
        }
    }
}

/// The cell behind one asynchronous `(function, arguments)` entry.
pub(super) struct AsyncMemoizeCell<V> {
    variants: Variants<V>,
    /// Serializes misses so concurrent callers with the same scope share one
    /// in-flight computation. Async so waiting callers do not block the
    /// executor.
    gate: tokio::sync::Mutex<()>,
    recursion: recursion::Guard,
}

impl<V> AsyncMemoizeCell<V> {
    /// Returns the value reusable from `cx`'s scope, awaiting `initialize`
    /// under a fresh tracker to compute a variant when there is none.
    ///
    /// Concurrent callers with the same scope share a single in-flight
    /// computation. `Marker` is the memoized function's type and only names
    /// it in the recursion panic.
    pub(super) async fn get_or_init<Marker, I>(&self, cx: &Cx, initialize: I) -> &V
    where
        I: AsyncFnOnce(&Cx) -> V,
    {
        if let Some(value) = self.variants.reuse(cx) {
            return value;
        }
        // Asserted before taking the gate so reentry panics instead of deadlocking on it.
        self.recursion.assert_not_recursive::<Marker>();
        let _gate = self.gate.lock().await;
        if let Some(value) = self.variants.reuse(cx) {
            return value;
        }
        let (child, tracker) = cx.track();
        let mut future = pin!(self.recursion.scope(|| initialize(&child)));
        // Clear ownership after every poll so sibling futures can wait on this cell and
        // the initializer can move between executor threads.
        let value = poll_fn(|task| self.recursion.scope(|| future.as_mut().poll(task))).await;
        self.variants.insert(cx, value, tracker.reads())
    }
}

// Not derived: the derive would needlessly bound `V: Default`.
impl<V> Default for AsyncMemoizeCell<V> {
    fn default() -> Self {
        Self {
            variants: Variants::default(),
            gate: tokio::sync::Mutex::new(()),
            recursion: recursion::Guard::default(),
        }
    }
}
