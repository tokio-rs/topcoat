use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::{Error, Result};

use crate::{
    View,
    buffer::{CellId, FrameId, ViewBuffer, ViewBufferScope},
    live::{Fill, Ticket, ViewHandle},
};

/// Collects a frame's live work: everything a component body or one run of a
/// reactive arm starts and must keep driving after its view is built.
///
/// One set exists per frame that can own live work. `'frame` is the owning
/// frame's lifetime: entries may borrow the frame's locals, which is legal
/// because the set is itself one of them and never escapes. Dropping the set
/// is one cancellation: the entries take every nested frame with them.
///
/// Entries are swept in order on every poll; there is no stored waker. The
/// driver re-polls the whole render while a pass made progress, so a fill in
/// one frame is observed by every frame that cares on the next sweep.
pub struct RefreshSet<'frame> {
    /// Boxed entries: fused render futures, node futures, and tenders.
    /// Completed entries are dropped.
    entries: Vec<Pin<Box<dyn Future<Output = Result<()>> + Send + 'frame>>>,
    /// This frame's index into the scope's ticket counters; the counters
    /// live on the scope because entries the set owns credit them.
    frame: FrameId,
    /// Cells spliced into this frame from elsewhere. The frame reports their
    /// posted errors and stays live until they retire. Fused cells do not
    /// belong here: a fused invocation's future is an entry of this same
    /// frame and is tracked directly.
    spliced: Vec<CellId>,
}

impl RefreshSet<'_> {
    /// Creates the set for one frame, registering the frame's ticket counter
    /// on the render scope.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    #[must_use]
    pub fn new() -> Self {
        Self::with_frame(ViewBufferScope::with(ViewBuffer::new_frame))
    }

    /// Creates the set for a frame registered earlier with
    /// [`new_frame`](crate::internal::new_frame).
    ///
    /// Generated code registers the frame and its slots and tickets before
    /// the set, so entries capture those values without borrowing anything
    /// the set outlives.
    #[must_use]
    pub fn with_frame(frame: FrameId) -> Self {
        Self {
            entries: Vec::new(),
            frame,
            spliced: Vec::new(),
        }
    }
}

impl<'frame> RefreshSet<'frame> {
    /// A fused component invocation: reserves the slot, writes a cell born
    /// started with that slot registered, and pushes the render future as a
    /// plain entry. Returns the placeholder to splice.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn invoke<F>(&mut self, render: impl FnOnce(Fill) -> F) -> View
    where
        F: Future<Output = Result<()>> + Send + 'frame,
    {
        let (view, fill) = self.reserve_invoke();
        self.entries.push(Box::pin(render(fill)));
        view
    }

    /// The bookkeeping half of [`invoke`](Self::invoke): reserves the slot
    /// and writes a cell born started with that slot registered, returning
    /// the placeholder and the fill, while the caller keeps the render
    /// future.
    ///
    /// This is how a loop batches: each iteration reserves through this, the
    /// iteration futures collect into one `Vec`, and a single pushed entry
    /// drives them all.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn reserve_invoke(&mut self) -> (View, Fill) {
        let frame = self.frame;
        ViewBufferScope::with(|buffer| {
            let (view, slot) = buffer.reserve_view();
            let cell = buffer.new_cell(true);
            buffer.consume(cell, slot, frame);
            (view, Fill::new(cell))
        })
    }

    /// The unfused split of [`invoke`](Self::invoke): parks the render
    /// future behind an unstarted cell, pushed as a tender, and returns the
    /// handle standing for the render.
    ///
    /// The render stays parked until a consumer starts it; when every handle
    /// clone drops first, it never runs.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn adopt<F>(&mut self, render: impl FnOnce(Fill) -> F) -> ViewHandle<'frame>
    where
        F: Future<Output = Result<()>> + Send + 'frame,
    {
        let cell = ViewBufferScope::with(|buffer| {
            let cell = buffer.new_cell(false);
            buffer.add_consumer(cell);
            cell
        });
        let future = render(Fill::new(cell));
        self.entries.push(Box::pin(tender(cell, future)));
        ViewHandle::new(cell)
    }

    /// Consumes a handle into this frame, bubble mode: reserves a slot,
    /// registers it with the handle's cell or serves it from the cached
    /// state, and signals start. Returns the placeholder to splice.
    ///
    /// The frame remembers the cell: it reports the cell's posted errors
    /// from [`barrier`](Self::barrier) and [`run`](Self::run), and stays
    /// live until the cell retires.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    // The handle is taken by value on purpose: splicing consumes one of the
    // cell's consumers, and the handle's drop is the bookkeeping.
    #[allow(clippy::needless_pass_by_value)]
    pub fn splice(&mut self, handle: ViewHandle<'_>) -> View {
        let frame = self.frame;
        let cell = handle.cell();
        let view = ViewBufferScope::with(|buffer| {
            let (view, slot) = buffer.reserve_view();
            buffer.consume(cell, slot, frame);
            view
        });
        self.spliced.push(cell);
        view
    }

    /// Registers other live work, such as a reactive node's future.
    pub fn push(&mut self, work: impl Future<Output = Result<()>> + Send + 'frame) {
        self.entries.push(Box::pin(work));
    }

    /// Adopts a handle created earlier with [`pending`]: the deferred half
    /// of [`adopt`](Self::adopt), pushing the parked render as a tender.
    ///
    /// A `view!` bound mid-body creates its handle and render future before
    /// the frame's set exists; the tail expansion attaches it here.
    pub fn attach<F>(&mut self, pending: Pending<F>)
    where
        F: Future<Output = Result<()>> + Send + 'frame,
    {
        self.entries
            .push(Box::pin(tender(pending.cell, pending.future)));
    }

    /// Counts a first-paint ticket for manually pushed work.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn ticket(&mut self) -> Ticket {
        let frame = self.frame;
        Ticket::new(
            frame,
            ViewBufferScope::with(|buffer| buffer.new_ticket(frame)),
        )
    }

    /// Polls every entry once, in order, dropping the completed ones.
    fn sweep(&mut self, task: &mut Context<'_>) -> Result<()> {
        let mut index = 0;
        while index < self.entries.len() {
            match self.entries[index].as_mut().poll(task) {
                Poll::Ready(Ok(())) => {
                    drop(self.entries.swap_remove(index));
                }
                Poll::Ready(Err(error)) => return Err(error),
                Poll::Pending => index += 1,
            }
        }
        Ok(())
    }

    /// Takes the first error a cell spliced into this frame posted.
    fn spliced_error(&self) -> Option<Error> {
        ViewBufferScope::with(|buffer| {
            self.spliced
                .iter()
                .find_map(|cell| buffer.take_cell_error(*cell))
        })
    }

    /// Drives the set until this frame's first-paint ticket count is zero:
    /// every child has handed its view over, even if work streams on inside.
    ///
    /// This is the join between a view's hoist and its burst, the successor
    /// of awaiting the children together.
    ///
    /// # Errors
    ///
    /// Returns the first error from an entry or a spliced cell.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub async fn barrier(&mut self) -> Result<()> {
        poll_fn(|task| {
            if let Err(error) = self.sweep(task) {
                return Poll::Ready(Err(error));
            }
            if let Some(error) = self.spliced_error() {
                return Poll::Ready(Err(error));
            }
            if ViewBufferScope::with(|buffer| buffer.ticket_count(self.frame)) == 0 {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    /// Drives the set until every entry is done and every spliced cell has
    /// retired: nothing in the frame can change anymore.
    ///
    /// An empty frame completes immediately, which is the non-streaming fast
    /// path.
    ///
    /// # Errors
    ///
    /// Returns the first error from an entry or a spliced cell.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub async fn run(mut self) -> Result<()> {
        poll_fn(move |task| {
            if let Err(error) = self.sweep(task) {
                return Poll::Ready(Err(error));
            }
            if let Some(error) = self.spliced_error() {
                return Poll::Ready(Err(error));
            }
            let retired = ViewBufferScope::with(|buffer| {
                self.spliced.iter().all(|cell| buffer.cell_retired(*cell))
            });
            if self.entries.is_empty() && retired {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

impl Default for RefreshSet<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A handle's render future between its creation and its adoption into a
/// frame's set.
///
/// Created by [`pending`] for a `view!` bound mid-body, before the frame's
/// set exists; [`RefreshSet::attach`] adopts it. Dropped unattached, the
/// render never runs, like any lazy handle whose consumers all drop.
pub struct Pending<F> {
    cell: CellId,
    future: F,
}

/// Creates a handle ahead of its owning set: the creation half of
/// [`RefreshSet::adopt`].
///
/// The render future is built immediately, so it may borrow anything in
/// scope at the binding; the returned [`Pending`] parks it until a set
/// adopts it with [`RefreshSet::attach`].
///
/// # Panics
///
/// Panics if no view is building on the current task.
pub fn pending<'frame, F>(render: impl FnOnce(Fill) -> F) -> (ViewHandle<'frame>, Pending<F>)
where
    F: Future<Output = Result<()>> + Send + 'frame,
{
    let cell = ViewBufferScope::with(|buffer| {
        let cell = buffer.new_cell(false);
        buffer.add_consumer(cell);
        cell
    });
    let future = render(Fill::new(cell));
    (ViewHandle::new(cell), Pending { cell, future })
}

/// Owns an adopted handle's render future.
///
/// Parks while the cell is unstarted, resolves as a no-op when every handle
/// clone drops first, and posts errors to the cell instead of failing
/// itself, so failures surface at the consumption sites; the tender's own
/// result is always `Ok`. When the render future finishes, the tender writes
/// the cell's terminal completed state, which is what retiring means.
async fn tender<F>(cell: CellId, future: F) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    let should_run = poll_fn(|_task| {
        ViewBufferScope::with(|buffer| {
            if buffer.cell_started(cell) {
                Poll::Ready(true)
            } else if buffer.cell_consumers(cell) == 0 {
                Poll::Ready(false)
            } else {
                // No waker: the driver re-polls while any pass progresses.
                Poll::Pending
            }
        })
    })
    .await;
    if !should_run {
        return Ok(());
    }
    match future.await {
        Ok(()) => ViewBufferScope::with(|buffer| buffer.complete(cell)),
        Err(error) => ViewBufferScope::with(|buffer| {
            buffer.post_error(cell, error);
            buffer.complete(cell);
        }),
    }
    Ok(())
}
