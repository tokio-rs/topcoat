use std::{cell::Cell, future::poll_fn, pin::pin};

use crate::render::Memory;

thread_local! {
    static MEMORY: Cell<Option<Memory>> = const { Cell::new(None) };
}

/// Returns whether an instruction memory is installed on the current task,
/// meaning an enclosing root `view!` invocation is building.
pub(crate) fn memory_installed() -> bool {
    let memory = MEMORY.take();
    let installed = memory.is_some();
    MEMORY.set(memory);
    installed
}

/// Runs `fut` with a fresh instruction [`Memory`] installed and returns the
/// memory alongside the output.
///
/// Every `view!` invocation inside `fut` appends to the installed memory.
/// The memory is only installed while `fut` polls on the task that entered
/// it; a future spawned onto another task builds its own memory instead.
/// Within `fut`, futures that build views may run concurrently, for example
/// under `try_join`.
pub(crate) async fn install<F: Future>(fut: F) -> (F::Output, Memory) {
    let mut fut = pin!(fut);
    let mut memory = Some(Memory::new());
    let output = poll_fn(|task_cx| {
        let _enter = Enter::new(&mut memory);
        fut.as_mut().poll(task_cx)
    })
    .await;
    let memory = memory.take().expect("the memory was reinstated on exit");
    (output, memory)
}

/// Runs `f` with a fresh instruction [`Memory`] installed and returns the
/// memory alongside the output.
///
/// The synchronous counterpart of [`install`], for captures that build in a
/// single burst outside any enclosing invocation.
pub(crate) fn install_sync<R>(f: impl FnOnce() -> R) -> (R, Memory) {
    let mut memory = Some(Memory::new());
    let output = {
        let _enter = Enter::new(&mut memory);
        f()
    };
    let memory = memory.take().expect("the memory was reinstated on exit");
    (output, memory)
}

/// Grants access to the installed memory for the duration of `f`.
///
/// The memory is taken out of its slot while `f` runs, so a re-entrant call
/// from inside `f` fails like a call outside any root `view!` invocation.
/// This keeps every borrow of the memory visible as a single synchronous
/// region.
///
/// # Panics
///
/// Panics if no memory is installed on the current task.
pub(crate) fn with_memory<R>(f: impl FnOnce(&mut Memory) -> R) -> R {
    let memory = MEMORY.take().unwrap_or_else(|| {
        panic!(
            "no view is building on the current task: build views with `view!`, \
             on the task that runs the outermost invocation"
        )
    });
    let mut restore = Restore(Some(memory));
    f(restore.0.as_mut().expect("memory was just stored"))
}

/// Puts the taken memory back into its slot, also when `f` panics.
struct Restore(Option<Memory>);

impl Drop for Restore {
    fn drop(&mut self) {
        MEMORY.set(self.0.take());
    }
}

/// Installs a memory for one poll of the wrapped future.
///
/// On creation the memory moves from the future's state into the thread
/// local slot, remembering whatever an enclosing invocation had installed.
/// On drop the memory moves back and the enclosing invocation's memory is
/// reinstated, also when the poll panics.
struct Enter<'a> {
    slot: &'a mut Option<Memory>,
    previous: Option<Memory>,
}

impl<'a> Enter<'a> {
    fn new(slot: &'a mut Option<Memory>) -> Self {
        let previous = MEMORY.replace(slot.take());
        Self { slot, previous }
    }
}

impl Drop for Enter<'_> {
    fn drop(&mut self) {
        *self.slot = MEMORY.replace(self.previous.take());
    }
}

/// Runs `fut` with a fresh instruction memory installed, discarding the
/// memory.
///
/// Unit tests use this to build views through the internal writer API
/// without a `view!` invocation establishing the memory.
#[cfg(test)]
pub(crate) async fn scope<F: Future>(fut: F) -> F::Output {
    install(fut).await.0
}
