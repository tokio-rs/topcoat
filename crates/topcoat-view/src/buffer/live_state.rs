use topcoat_core::error::Error;

use crate::{
    View,
    buffer::{Instruction, ViewBuffer, ViewSlot},
};

/// The index of a handle's cell in the buffer's live state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellId(usize);

/// The index of a frame's first-paint ticket counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameId(usize);

/// The index of a ticket's done flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TicketId(usize);

/// The lifecycle of the render behind a cell.
///
/// A cell moves forward through these states in order; a failure is tracked
/// separately, since it can follow any started state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    /// Created but not yet consumed anywhere; the render has not run.
    #[default]
    Unstarted,
    /// A consumer signalled start; the render is in flight.
    Started,
    /// The first state arrived and is cached for later splices.
    Delivered,
    /// The render future finished; nothing inside the cell can change.
    Completed,
}

/// A handle's cell: the lifecycle of the render behind it, the cached first
/// state, the failure it reported, and the consumers and slots observing it.
#[derive(Debug, Default)]
struct CellRecord {
    lifecycle: Lifecycle,
    /// The delivered first state, served to splices that arrive later.
    state: Option<View>,
    /// Whether the render behind the cell failed.
    failed: bool,
    /// The reported failure, held until a consuming frame takes it.
    error: Option<Error>,
    /// Live handle clones. A count of zero while the cell is still unstarted
    /// means the render never runs.
    consumers: usize,
    /// Slots waiting on the first state, each with the frame whose ticket
    /// counter to credit at delivery.
    waiting: Vec<(ViewSlot, FrameId)>,
}

/// The live-render state of a build: handle cells, per-frame first-paint
/// ticket counters, and the flags the driver sweeps on.
///
/// The state lives on the [`ViewBuffer`] so it travels with the buffer
/// through the scope install around each poll; the render future itself
/// holds only `Copy` indices into it. Ticket counters live here rather than
/// on the `RefreshSet` because the code that credits them runs inside
/// entries the set owns, and safe Rust does not let an entry hold a mutable
/// path into its owner.
#[derive(Debug, Default)]
pub(crate) struct LiveState {
    /// Set by a refill; the driver sends a chunk when it is set.
    dirty: bool,
    /// Set by any fill or state change; a sweep runs until a pass leaves it
    /// unset.
    progress: bool,
    cells: Vec<CellRecord>,
    ticket_counts: Vec<usize>,
    ticket_done: Vec<bool>,
}

/// The fallback error a frame reports when a view spliced into it failed
/// and the original error was already taken by another consuming frame.
#[derive(Debug)]
struct SplicedViewFailed;

impl std::fmt::Display for SplicedViewFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a view spliced into this frame failed to render")
    }
}

impl std::error::Error for SplicedViewFailed {}

impl ViewBuffer {
    /// Registers a frame and returns the index of its ticket counter.
    pub(crate) fn new_frame(&mut self) -> FrameId {
        self.live.ticket_counts.push(0);
        FrameId(self.live.ticket_counts.len() - 1)
    }

    /// Returns the number of outstanding first-paint tickets of a frame.
    pub(crate) fn ticket_count(&self, frame: FrameId) -> usize {
        self.live.ticket_counts[frame.0]
    }

    /// Counts a first-paint ticket for a frame and returns its done flag's
    /// index.
    ///
    /// The done flag lives here so the ticket value itself stays `Copy` and
    /// [`hand_over`](Self::hand_over) stays idempotent across arm runs.
    pub(crate) fn new_ticket(&mut self, frame: FrameId) -> TicketId {
        self.live.ticket_counts[frame.0] += 1;
        self.live.ticket_done.push(false);
        TicketId(self.live.ticket_done.len() - 1)
    }

    /// Credits a ticket back to its frame, once; later calls are no-ops.
    pub(crate) fn hand_over(&mut self, frame: FrameId, ticket: TicketId) {
        if !self.live.ticket_done[ticket.0] {
            self.live.ticket_done[ticket.0] = true;
            self.live.ticket_counts[frame.0] -= 1;
            self.live.progress = true;
        }
    }

    /// Registers a cell, born started for a fused invocation and unstarted
    /// for an adopted handle.
    pub(crate) fn new_cell(&mut self, started: bool) -> CellId {
        self.live.cells.push(CellRecord {
            lifecycle: if started {
                Lifecycle::Started
            } else {
                Lifecycle::Unstarted
            },
            ..CellRecord::default()
        });
        CellId(self.live.cells.len() - 1)
    }

    /// Counts a live handle clone on a cell.
    pub(crate) fn add_consumer(&mut self, cell: CellId) {
        self.live.cells[cell.0].consumers += 1;
    }

    /// Releases a handle clone; dropping to zero while still unstarted is
    /// what lets a parked render resolve as a no-op.
    pub(crate) fn remove_consumer(&mut self, cell: CellId) {
        let record = &mut self.live.cells[cell.0];
        record.consumers = record.consumers.saturating_sub(1);
        self.live.progress = true;
    }

    /// Returns whether a consumer has signalled the cell's render to start.
    pub(crate) fn cell_started(&self, cell: CellId) -> bool {
        self.live.cells[cell.0].lifecycle != Lifecycle::Unstarted
    }

    /// Signals a cell's render to start without registering a slot, for a
    /// consuming construct that receives the states instead of splicing.
    pub(crate) fn start_cell(&mut self, cell: CellId) {
        let record = &mut self.live.cells[cell.0];
        if record.lifecycle == Lifecycle::Unstarted {
            record.lifecycle = Lifecycle::Started;
            self.live.progress = true;
        }
    }

    /// Returns whether the cell's render reported a failure.
    pub(crate) fn cell_failed(&self, cell: CellId) -> bool {
        self.live.cells[cell.0].failed
    }

    /// Returns the number of live handle clones on a cell.
    pub(crate) fn cell_consumers(&self, cell: CellId) -> usize {
        self.live.cells[cell.0].consumers
    }

    /// Consumes a cell into a reserved slot on behalf of a frame.
    ///
    /// Signals start, then serves the slot from the cached state when the
    /// first state already arrived; otherwise the slot waits, and the frame's
    /// ticket counter is credited back at delivery. A cell that failed
    /// before delivering leaves the slot pending; the consuming frame
    /// reports the failure through its spliced list.
    pub(crate) fn consume(&mut self, cell: CellId, slot: ViewSlot, frame: FrameId) {
        self.live.progress = true;
        let record = &mut self.live.cells[cell.0];
        if record.lifecycle == Lifecycle::Unstarted {
            record.lifecycle = Lifecycle::Started;
        }
        let state = record.state.clone();
        match state {
            Some(view) => self.fill_view(slot, view),
            None if record.failed => {}
            None => {
                record.waiting.push((slot, frame));
                self.live.ticket_counts[frame.0] += 1;
            }
        }
    }

    /// Delivers a cell's first state: caches it, fills every waiting slot,
    /// and credits the waiting frames' tickets.
    pub(crate) fn deliver(&mut self, cell: CellId, view: View) {
        self.live.progress = true;
        self.live.dirty = true;
        let waiting = std::mem::take(&mut self.live.cells[cell.0].waiting);
        for (slot, frame) in waiting {
            self.fill_view(slot, view.clone());
            let count = &mut self.live.ticket_counts[frame.0];
            *count = count.saturating_sub(1);
        }
        let record = &mut self.live.cells[cell.0];
        record.state = Some(view);
        record.lifecycle = Lifecycle::Delivered;
    }

    /// Posts a failure to a cell.
    ///
    /// Waiting frames get their tickets back so their barriers can observe
    /// the error; waiting slots stay pending, since the region they sit in
    /// is replaced by whoever catches.
    pub(crate) fn post_error(&mut self, cell: CellId, error: Error) {
        self.live.progress = true;
        let waiting = std::mem::take(&mut self.live.cells[cell.0].waiting);
        for (_slot, frame) in waiting {
            let count = &mut self.live.ticket_counts[frame.0];
            *count = count.saturating_sub(1);
        }
        let record = &mut self.live.cells[cell.0];
        record.failed = true;
        if record.error.is_none() {
            record.error = Some(error);
        }
    }

    /// Writes a cell's terminal state: its render future finished.
    pub(crate) fn complete(&mut self, cell: CellId) {
        self.live.progress = true;
        self.live.cells[cell.0].lifecycle = Lifecycle::Completed;
    }

    /// Takes a failed cell's error for the consuming frame reporting it.
    ///
    /// The first taker gets the posted error; later takers, other frames the
    /// cell was spliced into, get a fallback error standing for the same
    /// failure.
    pub(crate) fn take_cell_error(&mut self, cell: CellId) -> Option<Error> {
        let record = &mut self.live.cells[cell.0];
        if !record.failed {
            return None;
        }
        Some(
            record
                .error
                .take()
                .unwrap_or_else(|| SplicedViewFailed.into()),
        )
    }

    /// Returns whether nothing inside the cell can change anymore.
    pub(crate) fn cell_retired(&self, cell: CellId) -> bool {
        let record = &self.live.cells[cell.0];
        record.lifecycle == Lifecycle::Completed || record.failed
    }

    /// Returns the cell's cached first state, if it was delivered.
    pub(crate) fn delivered_view(&self, cell: CellId) -> Option<View> {
        self.live.cells[cell.0].state.clone()
    }

    /// Redirects a slot to `view`, resolving its placeholder or replacing
    /// its previous fill, and marks the buffer dirty.
    ///
    /// The live counterpart of [`fill_view`](Self::fill_view): the first
    /// fill of a reactive node's slot and every swap after it go through
    /// here.
    ///
    /// # Panics
    ///
    /// Panics if the slot was reserved in a different buffer or if the view
    /// was built in a different, still building buffer.
    pub(crate) fn refill_view(&mut self, slot: ViewSlot, view: View) {
        assert!(
            slot.buffer() == self.id,
            "tried to refill a view slot outside the `view!` invocation it was reserved in",
        );
        let entry = self.view_block_entry(view);
        let instruction = self.instructions.fetch_mut(slot.ptr());
        debug_assert!(
            matches!(
                instruction,
                Instruction::Placeholder | Instruction::Jmp { .. }
            ),
            "tried to refill an instruction that is not a view slot",
        );
        *instruction = Instruction::Jmp { entry };
        self.live.dirty = true;
        self.live.progress = true;
    }

    /// Takes the dirty flag: whether a refill changed the document since the
    /// last take.
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.live.dirty)
    }

    /// Returns whether the current poll pass made progress.
    pub(crate) fn progress(&self) -> bool {
        self.live.progress
    }

    /// Clears the progress flag; the driver does this before each poll.
    pub(crate) fn clear_progress(&mut self) {
        self.live.progress = false;
    }
}
