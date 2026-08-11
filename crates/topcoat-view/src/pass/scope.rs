use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::Waker,
};

thread_local! {
    static CURRENT: Cell<Option<Box<PassState>>> = const { Cell::new(None) };
}

/// The state of one request's render passes.
///
/// Owned by the driver between polls and installed on the thread for the
/// duration of each poll, the same shape the view buffer scope uses, so a
/// request task stays free to migrate between worker threads.
#[derive(Default)]
pub(crate) struct PassState {
    /// The current pass number. Zero before the first pass.
    pub(crate) pass: u64,
    /// The request task's waker, refreshed by the driver on every poll, so
    /// render code polling children inline registers the real waker.
    pub(crate) waker: Option<Waker>,
    /// Per component output slots, keyed by identity hash. An entry exists
    /// while its component is live.
    pub(crate) slots: HashMap<u128, Slot>,
    /// Components whose body has not finished its first render.
    pub(crate) births: HashSet<u128>,
    /// Caught errors waiting to be consumed by their catcher's next render,
    /// including unclaimed content errors. A pass does not seal while any
    /// remain.
    pub(crate) stashed: usize,
    /// Bookkeeping for anonymous content components, keyed by identity hash.
    pub(crate) content: HashMap<u128, ContentState>,
    /// Deferred futures registered during this poll, drained by the driver.
    pub(crate) deferred: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

/// One content component's placement and error state.
#[derive(Default)]
pub(crate) struct ContentState {
    /// The error that completed the content's future, until a placement or
    /// the owner claims it.
    pub(crate) error: Option<topcoat_core::error::Error>,
    /// The pass the error was recorded on. The owner only claims errors
    /// from earlier passes, so a placement later in the same pass wins.
    pub(crate) error_pass: u64,
    /// The pass the content was last placed on, enforcing one placement per
    /// pass.
    pub(crate) placed_pass: u64,
}

/// One component's output slot.
#[derive(Default)]
pub(crate) struct Slot {
    pub(crate) html: String,
    pub(crate) rendered_pass: u64,
}

/// Installs a [`PassState`] on the thread for the guard's lifetime.
///
/// Must never be held across a suspension point: the drop swap has to run on
/// the thread that installed, which the `!Send` marker enforces.
pub(crate) struct PassScope<'a> {
    slot: &'a mut Option<Box<PassState>>,
    _not_send: PhantomData<*const ()>,
}

impl<'a> PassScope<'a> {
    /// Swaps `slot`'s state onto the thread until the guard drops.
    pub(crate) fn install(slot: &'a mut Option<Box<PassState>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        PassScope {
            slot,
            _not_send: PhantomData,
        }
    }

    /// Runs `f` with the installed pass state.
    ///
    /// # Panics
    ///
    /// Panics if no pass is running on the current task.
    #[track_caller]
    pub(crate) fn with<R>(f: impl FnOnce(&mut PassState) -> R) -> R {
        Self::try_with(f).expect(
            "no render pass is running on the current task: streaming render \
             code only runs under a pass driver",
        )
    }

    /// Runs `f` with the installed pass state, if a pass is running.
    pub(crate) fn try_with<R>(f: impl FnOnce(&mut PassState) -> R) -> Option<R> {
        let mut state = CURRENT.replace(None)?;
        let result = f(&mut state);
        CURRENT.set(Some(state));
        Some(result)
    }
}

impl Drop for PassScope<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}
