//! The render scope: the one shared mutable home for the buffer, the
//! handle cells, and the ticket counters, reached through a thread-local
//! install around every poll.
//!
//! Topcoat-level API under check (DESIGN.md "The Handle Machinery"):
//!
//! - `RenderScope` is owned by the driver, installed into a thread local
//!   for the duration of each poll, and reached as `&mut` through
//!   [`with`]. No `Arc`, no `Mutex`, no atomics anywhere; the render
//!   future stays `Send` because the scope travels with the driver, not
//!   inside the futures.
//! - [`View`] is today's cheap, inert, `Copy` handle to an instruction
//!   block; blocks embed children by reference.
//! - [`ViewSlot::fill`]/[`ViewSlot::refill`]: a reserved hole filled
//!   through the scope; `refill` marks the buffer dirty, which is what
//!   the driver turns into a chunk.
//! - [`CellRecord`]: a handle's cell. Lifecycle Unstarted -> Started ->
//!   Delivered -> Completed, posted error transitions, the consumer
//!   count feeding the lazy no-op rule, and the slots waiting on the
//!   first state, each with the frame ticket to credit.
//! - First-paint tickets are per-frame counters ON THE SCOPE, not fields
//!   of the `RefreshSet`: the code that credits them runs inside entries
//!   the set owns, and safe Rust does not let an entry hold a mutable
//!   path into its owner.

use std::cell::RefCell;

pub type Error = String;
pub type Result<T = ()> = std::result::Result<T, Error>;

pub type SlotId = usize;
pub type BlockId = usize;
pub type CellId = usize;
pub type FrameId = usize;
pub type TicketId = usize;

#[derive(Clone)]
enum Node {
    Html(&'static str),
    Text(String),
    Block(BlockId),
    Slot(SlotId),
}

/// One piece of a block under construction; `Splice` embeds an already
/// rendered view by reference, which is how a parent's output points at
/// a child without containing it.
pub enum Part {
    Html(&'static str),
    Text(String),
    Splice(View),
}

/// Today's `View`: an inert `Copy` handle to a block.
#[derive(Clone, Copy)]
pub struct View(BlockId);

/// A reserved hole in the buffer.
#[derive(Clone, Copy)]
pub struct ViewSlot(SlotId);

impl ViewSlot {
    /// The swap: refilling after the first paint marks the buffer dirty
    /// so the driver sends a chunk. The first fill of a node's slot goes
    /// through here too; the driver ignores dirty until the first paint.
    pub fn refill(self, view: View) {
        with(|s| {
            s.slots[self.0] = Some(view);
            s.dirty = true;
            s.progress = true;
        });
    }
}

/// A handle's cell.
#[derive(Default)]
pub struct CellRecord {
    pub started: bool,
    /// The first state; `Err` is a failure before the fill.
    pub delivered: Option<std::result::Result<View, Error>>,
    /// An error transition posted after delivery.
    pub posted: Option<Error>,
    /// Terminal: the render future finished. Written by the tender;
    /// what "retired" means for `run` and `next_state`.
    pub completed: bool,
    /// Live handle clones; meaningful while unstarted (the lazy no-op
    /// rule: zero consumers while unstarted means the render never runs).
    pub consumers: usize,
    /// Slots waiting on delivery, each with the frame ticket to credit.
    waiting: Vec<(SlotId, FrameId)>,
}

#[derive(Default)]
pub struct RenderScope {
    blocks: Vec<Vec<Node>>,
    slots: Vec<Option<View>>,
    /// Set by `refill`/delivery; the driver sends a chunk when set.
    pub dirty: bool,
    /// Set by any fill or state change; the sweep runs until a pass
    /// leaves it unset.
    pub progress: bool,
    pub cells: Vec<CellRecord>,
    ticket_counts: Vec<usize>,
    ticket_done: Vec<bool>,
}

impl RenderScope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds one instruction block and returns its inert handle.
    pub fn build(&mut self, parts: Vec<Part>) -> View {
        let nodes = parts
            .into_iter()
            .map(|p| match p {
                Part::Html(h) => Node::Html(h),
                Part::Text(t) => Node::Text(t),
                Part::Splice(v) => Node::Block(v.0),
            })
            .collect();
        self.blocks.push(nodes);
        View(self.blocks.len() - 1)
    }

    /// Reserves a slot and returns the placeholder view embedding it.
    pub fn reserve(&mut self) -> (View, ViewSlot) {
        let sid = self.slots.len();
        self.slots.push(None);
        self.blocks.push(vec![Node::Slot(sid)]);
        (View(self.blocks.len() - 1), ViewSlot(sid))
    }

    pub fn new_frame(&mut self) -> FrameId {
        self.ticket_counts.push(0);
        self.ticket_counts.len() - 1
    }

    pub fn ticket_count(&self, frame: FrameId) -> usize {
        self.ticket_counts[frame]
    }

    /// A node's first-paint ticket: counted now, credited idempotently
    /// through [`RenderScope::hand_over`]. The done flag lives here, on
    /// the scope, so the `Ticket` value itself can stay `Copy` and be
    /// re-created by every arm run.
    pub fn new_ticket(&mut self, frame: FrameId) -> TicketId {
        self.ticket_counts[frame] += 1;
        self.ticket_done.push(false);
        self.ticket_done.len() - 1
    }

    pub fn hand_over(&mut self, frame: FrameId, ticket: TicketId) {
        if !self.ticket_done[ticket] {
            self.ticket_done[ticket] = true;
            self.ticket_counts[frame] -= 1;
            self.progress = true;
        }
    }

    pub fn new_cell(&mut self, started: bool) -> CellId {
        self.cells.push(CellRecord {
            started,
            ..CellRecord::default()
        });
        self.cells.len() - 1
    }

    /// A splice consuming a cell: registers the slot (or serves it from
    /// the cache when the first state already arrived), signals start,
    /// and counts a first-paint ticket for the consuming frame while
    /// the state is still pending.
    pub fn consume(&mut self, cell: CellId, slot: ViewSlot, frame: FrameId) {
        self.progress = true;
        self.cells[cell].started = true;
        match &self.cells[cell].delivered {
            Some(Ok(view)) => {
                let view = *view;
                self.slots[slot.0] = Some(view);
            }
            Some(Err(_)) => {
                // The consuming frame reports the error via its spliced
                // list; the slot stays pending and the region will be
                // replaced by whoever catches.
            }
            None => {
                self.ticket_counts[frame] += 1;
                self.cells[cell].waiting.push((slot.0, frame));
            }
        }
    }

    /// The fill: cache the first state, fill every waiting slot, credit
    /// the waiting tickets, wake the sweep.
    pub fn deliver(&mut self, cell: CellId, state: std::result::Result<View, Error>) {
        self.progress = true;
        self.dirty = true;
        let waiting = std::mem::take(&mut self.cells[cell].waiting);
        for (slot, frame) in waiting {
            if let Ok(view) = &state {
                self.slots[slot] = Some(*view);
            }
            self.ticket_counts[frame] = self.ticket_counts[frame].saturating_sub(1);
        }
        self.cells[cell].delivered = Some(state);
    }

    pub fn post_error(&mut self, cell: CellId, error: Error) {
        self.progress = true;
        if self.cells[cell].delivered.is_none() {
            self.deliver(cell, Err(error));
        } else {
            self.cells[cell].posted = Some(error);
        }
    }

    pub fn complete(&mut self, cell: CellId) {
        self.progress = true;
        self.cells[cell].completed = true;
    }

    pub fn cell_error(&self, cell: CellId) -> Option<Error> {
        match &self.cells[cell].delivered {
            Some(Err(e)) => Some(e.clone()),
            _ => self.cells[cell].posted.clone(),
        }
    }

    pub fn cell_retired(&self, cell: CellId) -> bool {
        self.cells[cell].completed || matches!(self.cells[cell].delivered, Some(Err(_)))
    }

    /// Executes the buffer from a root view: framework code, no user
    /// code, exactly what the driver does per chunk.
    pub fn render(&self, view: View) -> String {
        let mut out = String::new();
        self.render_block(view.0, &mut out);
        out
    }

    fn render_block(&self, block: BlockId, out: &mut String) {
        for node in &self.blocks[block] {
            match node {
                Node::Html(h) => out.push_str(h),
                Node::Text(t) => out.push_str(t),
                Node::Block(b) => self.render_block(*b, out),
                Node::Slot(sid) => match &self.slots[*sid] {
                    Some(v) => self.render_block(v.0, out),
                    None => out.push_str("<!--pending-->"),
                },
            }
        }
    }
}

thread_local! {
    static SCOPE: RefCell<Option<RenderScope>> = const { RefCell::new(None) };
}

/// The one door to the scope. Panics when no render is being polled.
pub fn with<R>(f: impl FnOnce(&mut RenderScope) -> R) -> R {
    SCOPE.with(|s| f(s.borrow_mut().as_mut().expect("render scope not installed")))
}

/// Tolerant variant for `Drop` impls: a `ViewHandle` can be dropped
/// after the driver stops polling (when the root future itself drops),
/// and consumer accounting no longer matters then.
pub fn try_with<R>(f: impl FnOnce(&mut RenderScope) -> R) -> Option<R> {
    SCOPE.with(|s| s.borrow_mut().as_mut().map(f))
}

/// Installs the scope around one poll: moved into the thread local,
/// moved back out after. The scope travels with the driver between
/// polls, which is what keeps the render `Send` without locks.
pub fn install<R>(scope: &mut RenderScope, f: impl FnOnce() -> R) -> R {
    SCOPE.with(|s| *s.borrow_mut() = Some(std::mem::take(scope)));
    let out = f();
    SCOPE.with(|s| *scope = s.borrow_mut().take().expect("render scope stolen"));
    out
}
