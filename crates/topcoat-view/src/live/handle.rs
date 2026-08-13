use std::{future::poll_fn, marker::PhantomData, task::Poll};

use topcoat_core::error::Result;

use crate::{
    View,
    buffer::{CellId, FrameId, TicketId, ViewBufferScope},
    live::Reactive,
};

/// What a component's render future fills instead of returning its view.
///
/// A live render future receives a `Fill` alongside its props, delivers its
/// first state through it, and keeps running; the view travels through the
/// fill, and the future's own output is the component's final status.
#[derive(Debug)]
pub struct Fill {
    cell: CellId,
}

impl Fill {
    pub(crate) fn new(cell: CellId) -> Self {
        Self { cell }
    }

    /// Delivers the first state: caches it in the handle's cell, fills every
    /// slot the handle was spliced into, and releases the waiting frames'
    /// first-paint tickets.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn fill(self, view: View) {
        ViewBufferScope::with(|buffer| buffer.deliver(self.cell, view));
    }
}

/// The reactive view handle: a cheap clone of one render of its content.
///
/// A handle stands for a render, it does not contain one. It is lazy: the
/// render runs once a view consumes the handle, and a handle whose clones
/// all drop unstarted never runs at all. Cloning shows the same render in
/// more than one place; the content renders exactly once.
///
/// `'frame` is the creating frame's lifetime: the render future borrows that
/// frame's locals, so the handle cannot outlive it. Deliberately `Clone` and
/// not `Copy`: clone and drop maintain the cell's consumer count, which is
/// how dropping every clone of an unstarted handle is observed.
#[derive(Debug)]
#[must_use = "a handle's render runs only once a view consumes it"]
pub struct ViewHandle<'frame> {
    cell: CellId,
    /// The last state this handle instance observed as a [`Reactive`], so
    /// each call to `next_state` yields the next one.
    observed: Observed,
    _frame: PhantomData<&'frame ()>,
}

/// How far a [`ViewHandle`] has read its cell's states.
#[derive(Debug, Clone, Copy)]
enum Observed {
    /// No state read yet; the first read signals start.
    Nothing,
    /// The delivered view was yielded; an error may still follow.
    Delivered,
    /// A failure was yielded, the handle's final state.
    Failed,
}

impl ViewHandle<'_> {
    pub(crate) fn new(cell: CellId) -> Self {
        Self {
            cell,
            observed: Observed::Nothing,
            _frame: PhantomData,
        }
    }

    pub(crate) fn cell(&self) -> CellId {
        self.cell
    }
}

impl Reactive for ViewHandle<'_> {
    type State = Result<View>;

    /// The first call signals the parked render to start and yields `Ok`
    /// with the view when the component hands it over, or `Err` when the
    /// render fails first. After an `Ok`, a failure climbing out of the
    /// component later arrives as an `Err` state; retirement follows the
    /// final state.
    async fn next_state(&mut self) -> Option<Result<View>> {
        let cell = self.cell;
        match self.observed {
            Observed::Nothing => {
                ViewBufferScope::with(|buffer| buffer.start_cell(cell));
                let state = poll_fn(|_task| {
                    ViewBufferScope::with(|buffer| {
                        if buffer.cell_failed(cell) {
                            let error = buffer
                                .take_cell_error(cell)
                                .expect("a failed cell reports an error");
                            return Poll::Ready(Err(error));
                        }
                        match buffer.delivered_view(cell) {
                            Some(view) => Poll::Ready(Ok(view)),
                            // No waker: the driver re-polls while any pass
                            // progresses.
                            None => Poll::Pending,
                        }
                    })
                })
                .await;
                self.observed = match state {
                    Ok(_) => Observed::Delivered,
                    Err(_) => Observed::Failed,
                };
                Some(state)
            }
            Observed::Delivered => {
                let error = poll_fn(|_task| {
                    ViewBufferScope::with(|buffer| {
                        if buffer.cell_failed(cell) {
                            let error = buffer
                                .take_cell_error(cell)
                                .expect("a failed cell reports an error");
                            return Poll::Ready(Some(error));
                        }
                        if buffer.cell_retired(cell) {
                            return Poll::Ready(None);
                        }
                        Poll::Pending
                    })
                })
                .await?;
                self.observed = Observed::Failed;
                Some(Err(error))
            }
            Observed::Failed => None,
        }
    }
}

impl Clone for ViewHandle<'_> {
    fn clone(&self) -> Self {
        ViewBufferScope::try_with(|buffer| buffer.add_consumer(self.cell));
        Self::new(self.cell)
    }
}

impl Drop for ViewHandle<'_> {
    fn drop(&mut self) {
        // Tolerant: the root future, and every handle inside it, is dropped
        // by the driver outside any poll, when no buffer is installed and
        // consumer accounting no longer matters.
        ViewBufferScope::try_with(|buffer| buffer.remove_consumer(self.cell));
    }
}

/// A first-paint ticket: one reserved spot in a frame's barrier.
///
/// A reactive node takes a ticket when it is registered and hands it over
/// when its first arm has rendered, which is what releases the frame's
/// barrier and, transitively, the first paint. `Copy`, so every arm run can
/// hold the same ticket; the done flag lives on the render scope, keeping
/// [`hand_over`](Self::hand_over) idempotent.
#[derive(Debug, Clone, Copy)]
pub struct Ticket {
    frame: FrameId,
    ticket: TicketId,
}

impl Ticket {
    pub(crate) fn new(frame: FrameId, ticket: TicketId) -> Self {
        Self { frame, ticket }
    }

    /// Credits the ticket back to its frame; later calls are no-ops.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn hand_over(self) {
        ViewBufferScope::with(|buffer| buffer.hand_over(self.frame, self.ticket));
    }
}
