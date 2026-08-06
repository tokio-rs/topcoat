use std::{cell::Cell, future::poll_fn, pin::pin};

use crate::render::Memory;

thread_local! {
    static MEMORY: Cell<Option<Memory>> = const { Cell::new(None) };
}

/// Runs `fut` inside a new view scope.
///
/// A scope owns the instruction [`Memory`] that every `view!` invocation
/// inside `fut` appends to, and that rendering the resulting
/// [`View`](crate::View)s reads from. Wrap the code path that builds and
/// renders a response in a single scope; the memory is freed when the scope
/// ends.
///
/// Views are bound to their scope: building or rendering one outside the
/// scope panics. In particular a view cannot travel into a spawned task,
/// because a scope covers only the future it wraps. Within the scope,
/// futures that build views may run concurrently, for example under
/// `try_join`.
pub async fn scope<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let mut memory = Some(Memory::new());
    poll_fn(move |task_cx| {
        let _enter = Enter::new(&mut memory);
        fut.as_mut().poll(task_cx)
    })
    .await
}

/// Grants access to the active scope's memory for the duration of `f`.
///
/// The memory is taken out of the scope slot while `f` runs, so a re-entrant
/// call from inside `f` fails like a call outside any scope. This keeps every
/// borrow of the memory visible as a single synchronous region.
///
/// # Panics
///
/// Panics if no scope is active on the current task.
pub(crate) fn with_memory<R>(f: impl FnOnce(&mut Memory) -> R) -> R {
    let memory = MEMORY.take().unwrap_or_else(|| {
        panic!(
            "no view scope is active: build and render views inside \
             `topcoat::view::scope`, on the task that entered it"
        )
    });
    let mut restore = Restore(Some(memory));
    f(restore.0.as_mut().expect("memory was just stored"))
}

/// Puts the taken memory back into the scope slot, also when `f` panics.
struct Restore(Option<Memory>);

impl Drop for Restore {
    fn drop(&mut self) {
        MEMORY.set(self.0.take());
    }
}

/// Installs a scope's memory for one poll of the wrapped future.
///
/// On creation the memory moves from the scope future into the thread local
/// slot, remembering whatever an enclosing scope had installed. On drop the
/// memory moves back and the enclosing scope's memory is reinstated, also
/// when the poll panics.
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
