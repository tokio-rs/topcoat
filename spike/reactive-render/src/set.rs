//! The per-frame `RefreshSet`, the reactive contract, and the handle.
//!
//! Topcoat-level API under check (DESIGN.md "The `RefreshSet`", "The
//! Reactive Contract", "The Handle Machinery"):
//!
//! - `RefreshSet<'frame>`: one set per frame that can own live work, a
//!   plain `Vec` of boxed non-`'static` entries, swept in order; no
//!   `FuturesUnordered`, no stored wakers.
//! - `invoke`: the fused component invocation. Reserve the slot, write a
//!   cell born started, push the render future itself as an entry.
//! - `adopt` + `splice`: the unfused split. `adopt` parks the boxed
//!   future behind an unstarted cell as a tender; `splice` consumes a
//!   handle in whichever frame shows it.
//! - `ViewHandle<'frame>`: `Clone` but deliberately not `Copy`; clone
//!   and drop adjust the cell's consumer count, feeding the lazy no-op
//!   rule.
//! - `Reactive`, the one-method contract:
//!   `async fn next_state(&mut self) -> Option<State>`, with `Defer` as
//!   the first implementation (lazy: the first call gives the future its
//!   first poll and yields `Pending` or `Ready` without waiting).
//! - `run_node`: the uniform render-then-race loop. A new state drops
//!   the arm frame, which cancels the replaced subtree's work.
//!
//! Findings recorded while making this compile are listed in README.md.

use std::future::{poll_fn, Future};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::scope::{self, with, CellId, FrameId, Result, TicketId, View};

/// What a component's render future fills instead of returning its view.
pub struct Fill {
    cell: CellId,
}

impl Fill {
    /// Used by the driver to mint the root fill.
    pub fn for_cell(cell: CellId) -> Self {
        Fill { cell }
    }

    /// The yield: deliver the first state, then the future keeps going.
    pub fn fill(self, view: View) {
        with(|s| s.deliver(self.cell, Ok(view)));
    }
}

/// The reactive view handle: a cheap `Clone` index into the handle's
/// cell, carrying the creating frame's lifetime. Deliberately not
/// `Copy`: clone and drop maintain the consumer count.
pub struct ViewHandle<'frame> {
    cell: CellId,
    _frame: PhantomData<&'frame ()>,
}

impl Clone for ViewHandle<'_> {
    fn clone(&self) -> Self {
        scope::try_with(|s| s.cells[self.cell].consumers += 1);
        ViewHandle {
            cell: self.cell,
            _frame: PhantomData,
        }
    }
}

impl Drop for ViewHandle<'_> {
    fn drop(&mut self) {
        scope::try_with(|s| {
            s.cells[self.cell].consumers = s.cells[self.cell].consumers.saturating_sub(1);
            s.progress = true;
        });
    }
}

/// A node's first-paint ticket. `Copy`, so every arm run can hold one;
/// idempotence lives on the scope as a done flag.
#[derive(Clone, Copy)]
pub struct Ticket {
    frame: FrameId,
    id: TicketId,
}

impl Ticket {
    pub fn hand_over(self) {
        with(|s| s.hand_over(self.frame, self.id));
    }
}

/// One frame's live work: a component body or one run of a `live` arm.
pub struct RefreshSet<'frame> {
    /// Boxed entries: fused render futures, node futures, and tenders.
    /// Polled in order on every sweep; completed entries are dropped.
    entries: Vec<Pin<Box<dyn Future<Output = Result> + Send + 'frame>>>,
    /// This frame's ticket counter lives on the scope; see scope.rs.
    frame: FrameId,
    /// Cells spliced into this frame from elsewhere: the frame reports
    /// their posted errors and stays live until they retire.
    spliced: Vec<CellId>,
}

impl<'frame> RefreshSet<'frame> {
    pub fn new() -> Self {
        RefreshSet {
            entries: Vec::new(),
            frame: with(|s| s.new_frame()),
            spliced: Vec::new(),
        }
    }

    /// A fused component invocation: reserve the slot, write a cell born
    /// started with that slot registered, push the render future as a
    /// plain entry. No tender, no parked state.
    pub fn invoke<F>(&mut self, render: impl FnOnce(Fill) -> F) -> View
    where
        F: Future<Output = Result> + Send + 'frame,
    {
        let (view, slot) = with(|s| s.reserve());
        let cell = with(|s| s.new_cell(true));
        with(|s| s.consume(cell, slot, self.frame));
        // Not added to `spliced`: the render future is an entry of this
        // same frame, so completion and errors are tracked directly.
        // `spliced` is only for cells whose future lives elsewhere.
        self.entries.push(Box::pin(render(Fill { cell })));
        view
    }

    /// The unfused split of `invoke`: park the boxed future behind an
    /// unstarted cell, pushed as a tender.
    pub fn adopt<F>(&mut self, render: impl FnOnce(Fill) -> F) -> ViewHandle<'frame>
    where
        F: Future<Output = Result> + Send + 'frame,
    {
        let cell = with(|s| {
            let cell = s.new_cell(false);
            s.cells[cell].consumers = 1;
            cell
        });
        let future = render(Fill { cell });
        self.entries.push(Box::pin(tender(cell, future)));
        ViewHandle {
            cell,
            _frame: PhantomData,
        }
    }

    /// Consumes a handle here, bubble mode: reserve a slot, register it
    /// with the cell (or serve it from the cache), signal start, and
    /// remember the cell so this frame reports its errors and outlives
    /// its retirement.
    pub fn splice(&mut self, handle: ViewHandle<'_>) -> View {
        let (view, slot) = with(|s| s.reserve());
        with(|s| s.consume(handle.cell, slot, self.frame));
        self.spliced.push(handle.cell);
        view
    }

    /// Registers other live work: a reactive node's future.
    pub fn push(&mut self, work: impl Future<Output = Result> + Send + 'frame) {
        self.entries.push(Box::pin(work));
    }

    /// A first-paint ticket for manually pushed work.
    pub fn ticket(&mut self) -> Ticket {
        Ticket {
            frame: self.frame,
            id: with(|s| s.new_ticket(self.frame)),
        }
    }

    fn sweep(&mut self, cx: &mut Context<'_>) -> Result {
        let mut i = 0;
        while i < self.entries.len() {
            match self.entries[i].as_mut().poll(cx) {
                Poll::Ready(Ok(())) => {
                    drop(self.entries.swap_remove(i));
                }
                Poll::Ready(Err(e)) => return Err(e),
                Poll::Pending => i += 1,
            }
        }
        Ok(())
    }

    fn spliced_error(&self) -> Option<crate::scope::Error> {
        with(|s| self.spliced.iter().find_map(|c| s.cell_error(*c)))
    }

    /// Drives the set until this frame's ticket count is zero, reporting
    /// the first error from an entry or a spliced cell.
    pub async fn barrier(&mut self) -> Result {
        poll_fn(|cx| {
            if let Err(e) = self.sweep(cx) {
                return Poll::Ready(Err(e));
            }
            if let Some(e) = self.spliced_error() {
                return Poll::Ready(Err(e));
            }
            if with(|s| s.ticket_count(self.frame)) == 0 {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    /// Drives the set until every entry is done and every spliced cell
    /// has retired. Empty frame: completes immediately.
    pub async fn run(mut self) -> Result {
        poll_fn(move |cx| {
            if let Err(e) = self.sweep(cx) {
                return Poll::Ready(Err(e));
            }
            if let Some(e) = self.spliced_error() {
                return Poll::Ready(Err(e));
            }
            let retired = with(|s| self.spliced.iter().all(|c| s.cell_retired(*c)));
            if self.entries.is_empty() && retired {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

/// The tender: owns an unfused handle's render future. Parks while the
/// cell is unstarted, resolves as a no-op if every handle clone drops
/// first, and posts errors to the cell instead of failing itself, so
/// its own result is always `Ok`. Writes the terminal completed state.
async fn tender<F: Future<Output = Result>>(cell: CellId, future: F) -> Result {
    let should_run = poll_fn(|_| {
        with(|s| {
            let record = &s.cells[cell];
            if record.started {
                Poll::Ready(true)
            } else if record.consumers == 0 {
                Poll::Ready(false)
            } else {
                // No waker: the sweep re-polls when anything progresses.
                Poll::Pending
            }
        })
    })
    .await;
    if !should_run {
        return Ok(());
    }
    match future.await {
        Ok(()) => with(|s| s.complete(cell)),
        Err(e) => with(|s| {
            s.post_error(cell, e);
            s.complete(cell);
        }),
    }
    Ok(())
}

/// The two states of a deferred load.
pub enum Deferred<T> {
    Pending,
    Ready(T),
}

/// The one-method reactive contract. `Send` supertrait and an explicit
/// `impl Future + Send` return keep pushed node futures `Send`.
pub trait Reactive: Send {
    type State: Send;

    /// First call: the state to render now. Later calls: replacements.
    /// `None`: retired, no further change possible.
    fn next_state(&mut self) -> impl Future<Output = Option<Self::State>> + Send;
}

/// A deferred load: owns its future, lazy until consumed.
pub struct Defer<F: Future> {
    state: DeferState<F>,
}

enum DeferState<F: Future> {
    Fresh(Pin<Box<F>>),
    Waiting(Pin<Box<F>>),
    Done,
}

pub struct Cx {
    pub fail_orders: bool,
}

/// Wraps a future instead of awaiting it. Inside a view the macro
/// supplies `cx`; the spike passes it explicitly everywhere.
pub fn defer<F>(_cx: &Cx, future: F) -> Defer<F>
where
    F: Future + Send,
{
    Defer {
        state: DeferState::Fresh(Box::pin(future)),
    }
}

impl<F> Reactive for Defer<F>
where
    F: Future + Send,
    F::Output: Send,
{
    type State = Deferred<F::Output>;

    fn next_state(&mut self) -> impl Future<Output = Option<Self::State>> + Send {
        async move {
            match std::mem::replace(&mut self.state, DeferState::Done) {
                DeferState::Fresh(mut future) => {
                    // Laziness: this is the future's first poll, at
                    // consumption. One poll, no waiting, so data already
                    // at hand reports `Ready` on the first paint.
                    let first = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await;
                    match first {
                        Poll::Ready(value) => Some(Deferred::Ready(value)),
                        Poll::Pending => {
                            self.state = DeferState::Waiting(future);
                            Some(Deferred::Pending)
                        }
                    }
                }
                DeferState::Waiting(future) => Some(Deferred::Ready(future.await)),
                DeferState::Done => None,
            }
        }
    }
}

/// The uniform node loop: await a state, run the arms, race the shown
/// arm's remaining work against the next state. A new state drops the
/// arm frame, cancelling the replaced subtree's outstanding work.
///
/// Shape note (see README finding 3): `arms` is a plain `FnMut`
/// returning a `Send` future, not an `AsyncFnMut`, so the caller's
/// closure must move per-call copies (`Copy` refs, `Ticket`s, slots)
/// into each arm future.
pub async fn run_node<R, A, AF>(mut reactive: R, mut arms: A) -> Result
where
    R: Reactive,
    A: FnMut(R::State) -> AF + Send,
    AF: Future<Output = Result> + Send,
{
    let Some(mut state) = reactive.next_state().await else {
        return Ok(());
    };
    loop {
        enum Out<S> {
            New(Option<S>),
            ArmDone(Result),
        }
        let mut arm = std::pin::pin!(arms(state));
        let mut next = std::pin::pin!(reactive.next_state());
        let out = poll_fn(|cx| {
            if let Poll::Ready(s) = next.as_mut().poll(cx) {
                return Poll::Ready(Out::New(s));
            }
            if let Poll::Ready(result) = arm.as_mut().poll(cx) {
                return Poll::Ready(Out::ArmDone(result));
            }
            Poll::Pending
        })
        .await;
        match out {
            // A new state: dropping `arm` is the cancellation of the
            // replaced arm's frame and everything it started.
            Out::New(Some(new_state)) => {
                state = new_state;
            }
            // Retired: finish the shown arm's remaining live work.
            Out::New(None) => {
                drop(next);
                return arm.await;
            }
            // The arm finished first: keep listening for the next state.
            Out::ArmDone(result) => {
                result?;
                match next.await {
                    Some(new_state) => state = new_state,
                    None => return Ok(()),
                }
            }
        }
    }
}
