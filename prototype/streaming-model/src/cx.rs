use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Waker};

/// Identifies one component instance within a request.
pub type CompId = u64;

/// The request context.
///
/// Cheap to clone; all clones share one request's state. It carries the pass
/// counter, the per-component output slots, the registry of live components,
/// the queue of deferred futures, and an event log the tests assert against.
///
/// Deferred futures are `'static` here. The design bounds them by `&'cx Cx`;
/// safe plumbing for handing a `'cx` future from a render to the driver is part
/// of the pass protocol open question, so the prototype uses owned inputs.
#[derive(Clone)]
pub struct Cx {
    inner: Rc<CxInner>,
}

struct CxInner {
    pass: Cell<u64>,
    next_id: Cell<CompId>,
    outputs: RefCell<HashMap<CompId, String>>,
    live: RefCell<HashMap<CompId, Rc<CompState>>>,
    deferred: RefCell<Vec<Pin<Box<dyn Future<Output = ()>>>>>,
    births: RefCell<HashSet<CompId>>,
    stashed: Cell<usize>,
    log: RefCell<Vec<String>>,
}

impl Cx {
    pub fn new() -> Self {
        Cx {
            inner: Rc::new(CxInner {
                pass: Cell::new(0),
                next_id: Cell::new(1),
                outputs: RefCell::new(HashMap::new()),
                live: RefCell::new(HashMap::new()),
                deferred: RefCell::new(Vec::new()),
                births: RefCell::new(HashSet::new()),
                stashed: Cell::new(0),
                log: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The current pass number. Zero before the first pass.
    pub fn pass(&self) -> u64 {
        self.inner.pass.get()
    }

    /// Registers a component instance at the start of its body and returns the
    /// guard that keeps it live. Dropping the guard deregisters the component,
    /// which happens when its future is dropped.
    pub fn register(&self, id: CompId, name: &'static str) -> Mount {
        let state = Rc::new(CompState {
            id,
            name,
            rendered_pass: Cell::new(0),
        });
        self.inner.live.borrow_mut().insert(id, state.clone());
        self.log(format!("body:{name}"));
        Mount {
            cx: self.clone(),
            state,
        }
    }

    /// Stores the output of one render and marks the component as rendered for
    /// the current pass.
    pub fn finish_render(&self, mount: &Mount, html: String) {
        let pass = self.pass();
        self.inner.outputs.borrow_mut().insert(mount.state.id, html);
        mount.state.rendered_pass.set(pass);
        self.log(format!("render:{}:p{pass}", mount.state.name));
    }

    /// Queues a deferred future. The driver polls the queue between passes, so
    /// one pass observes a consistent snapshot of completions.
    pub fn push_deferred(&self, fut: Pin<Box<dyn Future<Output = ()>>>) {
        self.inner.deferred.borrow_mut().push(fut);
    }

    /// Polls every queued deferred future once. Called by the driver before a
    /// pass begins, never during one.
    pub fn poll_deferred(&self) {
        let mut pending = std::mem::take(&mut *self.inner.deferred.borrow_mut());
        let waker = Waker::noop();
        let mut ctx = Context::from_waker(waker);
        pending.retain_mut(|fut| fut.as_mut().poll(&mut ctx).is_pending());
        self.inner.deferred.borrow_mut().append(&mut pending);
    }

    /// The number of deferred futures still in flight.
    pub fn outstanding_deferred(&self) -> usize {
        self.inner.deferred.borrow().len()
    }

    /// Appends an event to the log.
    pub fn log(&self, event: impl Into<String>) {
        self.inner.log.borrow_mut().push(event.into());
    }

    /// A copy of the full event log.
    pub fn log_snapshot(&self) -> Vec<String> {
        self.inner.log.borrow().clone()
    }

    /// How many logged events start with the given prefix.
    pub fn log_count(&self, prefix: &str) -> usize {
        self.inner
            .log
            .borrow()
            .iter()
            .filter(|e| e.starts_with(prefix))
            .count()
    }

    pub(crate) fn advance_pass(&self) {
        self.inner.pass.set(self.inner.pass.get() + 1);
    }

    pub(crate) fn fresh_id(&self) -> CompId {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        id
    }

    pub(crate) fn rendered_this_pass(&self, id: CompId) -> bool {
        self.inner
            .live
            .borrow()
            .get(&id)
            .is_some_and(|st| st.rendered_pass.get() == self.pass())
    }

    pub(crate) fn birth_started(&self, id: CompId) {
        self.inner.births.borrow_mut().insert(id);
    }

    pub(crate) fn birth_done(&self, id: CompId) {
        self.inner.births.borrow_mut().remove(&id);
    }

    pub(crate) fn note_stashed(&self) {
        self.inner.stashed.set(self.inner.stashed.get() + 1);
    }

    pub(crate) fn consume_stashed(&self) {
        self.inner.stashed.set(self.inner.stashed.get() - 1);
    }

    pub(crate) fn stashed(&self) -> usize {
        self.inner.stashed.get()
    }

    pub(crate) fn births_in_flight(&self) -> usize {
        self.inner.births.borrow().len()
    }

    pub(crate) fn all_rendered(&self) -> bool {
        let pass = self.pass();
        self.inner
            .live
            .borrow()
            .values()
            .all(|st| st.rendered_pass.get() == pass)
    }

    pub(crate) fn outputs_snapshot(&self) -> HashMap<CompId, String> {
        self.inner.outputs.borrow().clone()
    }

    pub(crate) fn live_names(&self) -> HashMap<CompId, &'static str> {
        self.inner
            .live
            .borrow()
            .iter()
            .map(|(id, st)| (*id, st.name))
            .collect()
    }

    fn deregister(&self, state: &CompState) {
        self.inner.live.borrow_mut().remove(&state.id);
        self.inner.outputs.borrow_mut().remove(&state.id);
        self.inner.births.borrow_mut().remove(&state.id);
        self.log(format!("drop:{}", state.name));
    }
}

impl Default for Cx {
    fn default() -> Self {
        Cx::new()
    }
}

/// Per-instance state shared between a component and the harness.
pub struct CompState {
    id: CompId,
    name: &'static str,
    rendered_pass: Cell<u64>,
}

impl CompState {
    pub(crate) fn rendered_pass(&self) -> u64 {
        self.rendered_pass.get()
    }
}

/// The registration guard a component holds in its frame.
///
/// Its drop is the component leaving the request: eviction, an error unwind, or
/// the end of the stream all reach it by dropping the component future.
pub struct Mount {
    cx: Cx,
    state: Rc<CompState>,
}

impl Mount {
    pub(crate) fn state(&self) -> Rc<CompState> {
        self.state.clone()
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        self.cx.deregister(&self.state);
    }
}

/// Runs an interpolation position: user code compiled inside a closure that is
/// not async, which is how `view!` rejects `.await` in render code.
pub fn interp(out: &mut String, f: impl FnOnce() -> String) {
    out.push_str(&f());
}
