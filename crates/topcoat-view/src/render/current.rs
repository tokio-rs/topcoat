use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, render::Arena};

thread_local! {
    /// The arena of the build running on the current thread, if any.
    ///
    /// A root `view!` invocation owns an [`Arena`] and installs it here for
    /// exactly the duration of each of its polls, so everything that runs
    /// inside the poll, including nested invocations in component bodies,
    /// appends to the same arena. A future spawned onto another task is not
    /// polled inside that region and builds an arena of its own.
    static CURRENT: Cell<Option<Box<Arena>>> = const { Cell::new(None) };
}

/// Swaps a slot with the [`CURRENT`] arena on creation and back on drop,
/// also when the guarded region panics.
///
/// Both directions of the protocol are this one move. Installing passes a
/// slot holding a fresh arena, which parks whatever an enclosing invocation
/// had installed in the slot for the duration of the guard. Taking passes an
/// empty slot, which moves the installed arena out and leaves the thread
/// local empty, so a re-entrant access inside the guarded region fails like
/// an access outside any build.
struct Swap<'a> {
    slot: &'a mut Option<Box<Arena>>,
}

impl<'a> Swap<'a> {
    fn new(slot: &'a mut Option<Box<Arena>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        Self { slot }
    }

    /// Returns the arena the swap moved into the slot, if any.
    fn arena(&mut self) -> Option<&mut Arena> {
        self.slot.as_deref_mut()
    }
}

impl Drop for Swap<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}

/// Returns whether a build is running on the current thread, meaning an
/// enclosing `view!` invocation has its arena installed.
pub(crate) fn is_building() -> bool {
    let arena = CURRENT.take();
    let building = arena.is_some();
    CURRENT.set(arena);
    building
}

/// Grants access to the installed arena for the duration of `f`.
///
/// The arena is taken out of the thread local while `f` runs, so a
/// re-entrant call from inside `f` fails like a call outside any build.
/// This keeps every borrow of the arena visible as a single synchronous
/// region.
///
/// # Panics
///
/// Panics if no build is running on the current thread.
pub(crate) fn with_arena<R>(f: impl FnOnce(&mut Arena) -> R) -> R {
    let mut slot = None;
    let mut swap = Swap::new(&mut slot);
    let arena = swap.arena().unwrap_or_else(|| {
        panic!(
            "no view is building on the current task: build views with `view!`, \
             on the task that runs the outermost invocation"
        )
    });
    f(arena)
}

/// Runs `f` with a fresh arena installed and returns the arena alongside
/// the output.
///
/// The synchronous door into a build, for captures that happen in a single
/// burst outside any enclosing invocation. [`ArenaFuture`] is the
/// asynchronous counterpart.
pub(crate) fn enter_sync<R>(f: impl FnOnce() -> R) -> (R, Arena) {
    let mut slot = Some(Box::new(Arena::new()));
    let output = {
        let _swap = Swap::new(&mut slot);
        f()
    };
    let arena = slot.take().expect("the arena was swapped back on exit");
    (output, *arena)
}

pin_project! {
    /// The future of a root `view!` invocation, deciding at its first poll
    /// who owns the arena.
    ///
    /// With an arena already installed on the task, an enclosing invocation
    /// owns the build and the inner future polls through unwrapped.
    /// Otherwise this invocation is the root: a fresh arena is installed
    /// while the inner future polls, and the resulting view takes ownership
    /// of it. Within the inner future, futures that build views may run
    /// concurrently, for example under `try_join`; they interleave only at
    /// await points, so each still appends to the installed arena in
    /// synchronous bursts.
    pub(crate) struct ArenaFuture<F> {
        #[pin]
        fut: F,
        // The arena between polls; `None` while a poll has it installed,
        // and `None` forever when the role is `Nested`.
        arena: Option<Box<Arena>>,
        role: Role,
    }
}

/// The role of an [`ArenaFuture`], decided at its first poll.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Undecided,
    Root,
    Nested,
}

impl<F> ArenaFuture<F> {
    pub(crate) fn new(fut: F) -> Self {
        Self {
            fut,
            arena: None,
            role: Role::Undecided,
        }
    }
}

impl<F: Future<Output = Result<View>>> Future for ArenaFuture<F> {
    type Output = Result<View>;

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.role == Role::Undecided {
            *this.role = if is_building() {
                Role::Nested
            } else {
                *this.arena = Some(Box::new(Arena::new()));
                Role::Root
            };
        }
        let output = if *this.role == Role::Root {
            let _swap = Swap::new(this.arena);
            this.fut.poll(task_cx)
        } else {
            this.fut.poll(task_cx)
        };
        match output {
            Poll::Ready(view) => Poll::Ready(match this.arena.take() {
                Some(arena) => Ok(view?.seal(*arena)),
                None => view,
            }),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Runs `fut` with a fresh arena installed, discarding the arena.
///
/// Unit tests use this to build views through the internal writer API
/// without a `view!` invocation establishing the arena.
#[cfg(test)]
pub(crate) async fn scope<F: Future>(fut: F) -> F::Output {
    let mut fut = std::pin::pin!(fut);
    let mut arena = Some(Box::new(Arena::new()));
    std::future::poll_fn(|task_cx| {
        let _swap = Swap::new(&mut arena);
        fut.as_mut().poll(task_cx)
    })
    .await
}
