use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, render::Memory};

thread_local! {
    static MEMORY: Cell<Option<Box<Memory>>> = const { Cell::new(None) };
}

/// Returns whether a`Memory` is installed on the current task,
/// meaning an enclosing `view!` invocation is building.
pub(crate) fn memory_installed() -> bool {
    let memory = MEMORY.take();
    let installed = memory.is_some();
    MEMORY.set(memory);
    installed
}

pin_project! {
    /// The future of a root `view!` invocation, deciding at its first poll
    /// who owns the instruction memory.
    ///
    /// With a memory already installed on the task, an enclosing invocation
    /// owns the build and the inner future polls through unwrapped.
    /// Otherwise this invocation is the root: a fresh memory is installed
    /// while the inner future polls, and the resulting view takes ownership
    /// of it. The memory is only installed on the task polling this future;
    /// a future spawned onto another task builds its own memory instead.
    /// Within the inner future, futures that build views may run
    /// concurrently, for example under `try_join`.
    pub struct RootView<F> {
        #[pin]
        fut: F,
        // The fresh memory between polls; `None` while a poll has it
        // installed, and `None` forever when an enclosing invocation owns
        // the build.
        memory: Option<Box<Memory>>,
        state: RootViewState,
    }
}

/// Tracks whether a [`RootView`] has decided who owns the memory, and the
/// decision it made.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RootViewState {
    Undecided,
    Installing,
    PollingThrough,
}

impl<F> RootView<F> {
    pub(crate) fn new(fut: F) -> Self {
        Self {
            fut,
            memory: None,
            state: RootViewState::Undecided,
        }
    }
}

impl<F: Future<Output = Result<View>>> Future for RootView<F> {
    type Output = Result<View>;

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.state == RootViewState::Undecided {
            *this.state = if memory_installed() {
                RootViewState::PollingThrough
            } else {
                *this.memory = Some(Box::new(Memory::new()));
                RootViewState::Installing
            };
        }
        let output = if *this.state == RootViewState::Installing {
            let _enter = Enter::new(this.memory);
            this.fut.poll(task_cx)
        } else {
            this.fut.poll(task_cx)
        };
        match output {
            Poll::Ready(view) => Poll::Ready(match this.memory.take() {
                Some(memory) => Ok(view?.into_owned(*memory)),
                None => view,
            }),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Runs `f` with a fresh instruction [`Memory`] installed and returns the
/// memory alongside the output.
///
/// The synchronous counterpart of [`RootView`], for captures that build in a
/// single burst outside any enclosing invocation.
pub(crate) fn install_sync<R>(f: impl FnOnce() -> R) -> (R, Memory) {
    let mut memory = Some(Box::new(Memory::new()));
    let output = {
        let _enter = Enter::new(&mut memory);
        f()
    };
    let memory = memory.take().expect("the memory was reinstated on exit");
    (output, *memory)
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
struct Restore(Option<Box<Memory>>);

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
    slot: &'a mut Option<Box<Memory>>,
    previous: Option<Box<Memory>>,
}

impl<'a> Enter<'a> {
    fn new(slot: &'a mut Option<Box<Memory>>) -> Self {
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
    let mut fut = std::pin::pin!(fut);
    let mut memory = Some(Box::new(Memory::new()));
    std::future::poll_fn(|task_cx| {
        let _enter = Enter::new(&mut memory);
        fut.as_mut().poll(task_cx)
    })
    .await
}
