use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use crate::buffer::ViewBuffer;

thread_local! {
    /// The buffer of the build running on the current thread, if any.
    ///
    /// A root `view!` invocation owns a [`ViewBuffer`] and installs it here
    /// for exactly the duration of each of its polls, so everything that runs
    /// inside the poll, including nested invocations in component bodies,
    /// appends to the same buffer. A future spawned onto another task is not
    /// polled inside that region and builds a buffer of its own.
    static CURRENT: Cell<Option<Box<ViewBuffer>>> = const { Cell::new(None) };
}

/// A region of a task with a buffer installed: the scope views are built in.
///
/// The associated functions are the only doors into the scope.
/// [`scope`](Self::scope) opens one around a root `view!` invocation's
/// future, [`scope_sync`](Self::scope_sync) around a synchronous build, and
/// [`with`](Self::with) grants access to the installed buffer.
///
/// An instance is the guard of one such region: creating it swaps a slot
/// with the thread local buffer and dropping it swaps back, also when the
/// region panics. Both directions of the protocol are this one move.
/// Installing passes a slot holding a fresh buffer, which parks whatever an
/// enclosing invocation had installed for the duration of the guard. Taking
/// passes an empty slot, which moves the installed buffer out, so a
/// re-entrant access inside the region fails like an access outside any
/// scope.
pub struct ViewBufferScope<'a> {
    slot: &'a mut Option<Box<ViewBuffer>>,
}

impl<'a> ViewBufferScope<'a> {
    fn swap(slot: &'a mut Option<Box<ViewBuffer>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        Self { slot }
    }

    /// Returns the buffer the guard moved into the slot, if any.
    fn buffer(&mut self) -> Option<&mut ViewBuffer> {
        self.slot.as_deref_mut()
    }

    /// Returns whether a scope is active on the current thread, meaning an
    /// enclosing invocation has its buffer installed.
    fn is_active() -> bool {
        let buffer = CURRENT.take();
        let active = buffer.is_some();
        CURRENT.set(buffer);
        active
    }

    /// Runs `fut` in a scope, deciding at its first poll who owns the buffer.
    ///
    /// With a scope already active on the task, an enclosing invocation owns
    /// the build and the future polls through unwrapped. Otherwise this
    /// invocation is the root: a fresh buffer is installed while the future
    /// polls and returned alongside the output, so the caller can seal the
    /// views built in it. Within the future, futures that build views may
    /// run concurrently, for example under `try_join`; they interleave only
    /// at await points, so each still appends to the installed buffer in
    /// synchronous bursts.
    pub fn scope<F: Future>(fut: F) -> impl Future<Output = (F::Output, Option<ViewBuffer>)> {
        ScopeFuture {
            fut,
            buffer: None,
            role: Role::Undecided,
        }
    }

    /// Runs `f` inside the active scope, or inside a fresh one opened for
    /// exactly its duration when none is active.
    ///
    /// Returns the fresh scope's buffer alongside the output, so the caller
    /// can seal the views built in it; `None` when an enclosing scope was
    /// active and `f` appended to its buffer. The synchronous counterpart of
    /// [`scope`](Self::scope), for builds that happen in a single burst.
    pub fn scope_sync<R>(f: impl FnOnce() -> R) -> (R, Option<ViewBuffer>) {
        if Self::is_active() {
            return (f(), None);
        }
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let output = {
            let _scope = ViewBufferScope::swap(&mut slot);
            f()
        };
        let buffer = slot.expect("the buffer was swapped back on exit");
        (output, Some(*buffer))
    }

    /// Grants access to the installed buffer for the duration of `f`.
    ///
    /// The buffer is taken out of the thread local while `f` runs, so a
    /// re-entrant call from inside `f` fails like a call outside any scope.
    /// This keeps every borrow of the buffer visible as a single synchronous
    /// region.
    ///
    /// # Panics
    ///
    /// Panics if no scope is active on the current thread.
    pub fn with<R>(f: impl FnOnce(&mut ViewBuffer) -> R) -> R {
        let mut slot = None;
        let mut scope = ViewBufferScope::swap(&mut slot);
        let buffer = scope.buffer().unwrap_or_else(|| {
            panic!(
                "no view is building on the current task: build views with `view!`, \
                 on the task that runs the outermost invocation"
            )
        });
        f(buffer)
    }
}

impl Drop for ViewBufferScope<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}

pin_project! {
    /// The future behind [`ViewBufferScope::scope`].
    struct ScopeFuture<F> {
        #[pin]
        fut: F,
        // The buffer between polls; `None` while a poll has it installed,
        // and `None` forever when the role is `Nested`.
        buffer: Option<Box<ViewBuffer>>,
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

impl<F: Future> Future for ScopeFuture<F> {
    type Output = (F::Output, Option<ViewBuffer>);

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.role == Role::Undecided {
            *this.role = if ViewBufferScope::is_active() {
                Role::Nested
            } else {
                *this.buffer = Some(Box::new(ViewBuffer::new()));
                Role::Root
            };
        }
        let output = if *this.role == Role::Root {
            let _scope = ViewBufferScope::swap(this.buffer);
            this.fut.poll(task_cx)
        } else {
            this.fut.poll(task_cx)
        };
        match output {
            Poll::Ready(output) => Poll::Ready((output, this.buffer.take().map(|buffer| *buffer))),
            Poll::Pending => Poll::Pending,
        }
    }
}
