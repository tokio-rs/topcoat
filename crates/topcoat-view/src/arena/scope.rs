use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, arena::Arena};

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

/// A region of a task with an arena installed: the scope views are built in.
///
/// The associated functions are the only doors into the scope.
/// [`scope`](Self::scope) opens one around a root `view!` invocation's
/// future, [`enter`](Self::enter) around a synchronous build, and
/// [`with`](Self::with) grants access to the installed arena.
///
/// An instance is the guard of one such region: creating it swaps a slot
/// with the thread local arena and dropping it swaps back, also when the
/// region panics. Both directions of the protocol are this one move.
/// Installing passes a slot holding a fresh arena, which parks whatever an
/// enclosing invocation had installed for the duration of the guard. Taking
/// passes an empty slot, which moves the installed arena out, so a
/// re-entrant access inside the region fails like an access outside any
/// scope.
pub struct ArenaScope<'a> {
    slot: &'a mut Option<Box<Arena>>,
}

impl<'a> ArenaScope<'a> {
    fn swap(slot: &'a mut Option<Box<Arena>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        Self { slot }
    }

    /// Returns the arena the guard moved into the slot, if any.
    fn arena(&mut self) -> Option<&mut Arena> {
        self.slot.as_deref_mut()
    }

    /// Returns whether a scope is active on the current thread, meaning an
    /// enclosing invocation has its arena installed.
    fn is_active() -> bool {
        let arena = CURRENT.take();
        let active = arena.is_some();
        CURRENT.set(arena);
        active
    }

    /// Runs a root `view!` invocation's future in a scope, deciding at its
    /// first poll who owns the arena.
    ///
    /// With a scope already active on the task, an enclosing invocation owns
    /// the build and the future polls through unwrapped. Otherwise this
    /// invocation is the root: a fresh arena is installed while the future
    /// polls, and the resulting view takes ownership of it. Within the
    /// future, futures that build views may run concurrently, for example
    /// under `try_join`; they interleave only at await points, so each still
    /// appends to the installed arena in synchronous bursts.
    pub fn scope(fut: impl Future<Output = Result<View>>) -> impl Future<Output = Result<View>> {
        ScopeFuture {
            fut,
            arena: None,
            role: Role::Undecided,
        }
    }

    /// Runs `f` inside the active scope, or inside a fresh one opened for
    /// exactly its duration when none is active.
    ///
    /// Returns the fresh scope's arena alongside the output, so the caller
    /// can seal the views built in it; `None` when an enclosing scope was
    /// active and `f` appended to its arena. The synchronous counterpart of
    /// [`scope`](Self::scope), for captures that happen in a single burst.
    pub fn enter<R>(f: impl FnOnce() -> R) -> (R, Option<Arena>) {
        if Self::is_active() {
            return (f(), None);
        }
        let mut slot = Some(Box::new(Arena::new()));
        let output = {
            let _scope = ArenaScope::swap(&mut slot);
            f()
        };
        let arena = slot.expect("the arena was swapped back on exit");
        (output, Some(*arena))
    }

    /// Grants access to the installed arena for the duration of `f`.
    ///
    /// The arena is taken out of the thread local while `f` runs, so a
    /// re-entrant call from inside `f` fails like a call outside any scope.
    /// This keeps every borrow of the arena visible as a single synchronous
    /// region.
    ///
    /// # Panics
    ///
    /// Panics if no scope is active on the current thread.
    pub fn with<R>(f: impl FnOnce(&mut Arena) -> R) -> R {
        let mut slot = None;
        let mut scope = ArenaScope::swap(&mut slot);
        let arena = scope.arena().unwrap_or_else(|| {
            panic!(
                "no view is building on the current task: build views with `view!`, \
                 on the task that runs the outermost invocation"
            )
        });
        f(arena)
    }

    /// Runs `fut` in a fresh scope, discarding the arena.
    ///
    /// Unit tests use this to build views through the internal writer API
    /// without a `view!` invocation establishing the scope.
    #[cfg(test)]
    pub async fn test<F: Future>(fut: F) -> F::Output {
        let mut fut = std::pin::pin!(fut);
        let mut arena = Some(Box::new(Arena::new()));
        std::future::poll_fn(|task_cx| {
            let _scope = ArenaScope::swap(&mut arena);
            fut.as_mut().poll(task_cx)
        })
        .await
    }
}

impl Drop for ArenaScope<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}

pin_project! {
    /// The future behind [`ArenaScope::scope`].
    struct ScopeFuture<F> {
        #[pin]
        fut: F,
        // The arena between polls; `None` while a poll has it installed,
        // and `None` forever when the role is `Nested`.
        arena: Option<Box<Arena>>,
        role: Role,
    }
}

/// The role of a [`ScopeFuture`], decided at its first poll.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Undecided,
    Root,
    Nested,
}

impl<F: Future<Output = Result<View>>> Future for ScopeFuture<F> {
    type Output = Result<View>;

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.role == Role::Undecided {
            *this.role = if ArenaScope::is_active() {
                Role::Nested
            } else {
                *this.arena = Some(Box::new(Arena::new()));
                Role::Root
            };
        }
        let output = if *this.role == Role::Root {
            let _scope = ArenaScope::swap(this.arena);
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
