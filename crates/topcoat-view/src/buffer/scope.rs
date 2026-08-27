use std::cell::Cell;

use crate::{PartsWriter, ViewHandle, buffer::ViewBuffer};

thread_local! {
    /// The buffer of the build running on the current thread, if any.
    ///
    /// The outermost view of a build owns a [`ViewBuffer`] and installs it
    /// here for exactly the duration of each of its polls, so everything
    /// that runs inside the poll, including the views nested in component
    /// bodies, appends to the same buffer. A future spawned onto another
    /// task is not polled inside that region and starts a build of its own.
    static CURRENT: Cell<Option<Box<ViewBuffer>>> = const { Cell::new(None) };
}

/// A region of a task with a buffer installed: the scope views are built in.
///
/// The associated functions are the only doors into the scope.
/// [`install`](Self::install) opens one around a poll of the outermost
/// view, [`with`](Self::with) grants access to the installed buffer, and
/// [`block`](Self::block) appends one view's instruction block to it.
///
/// An instance is the guard of one such region: creating it swaps a slot
/// with the thread local buffer and dropping it swaps back, also when the
/// region panics. Both directions of the protocol are this one move.
/// Installing passes a slot holding a buffer, which parks whatever an
/// enclosing build had installed for the duration of the guard. Taking
/// passes an empty slot, which moves the installed buffer out, so a
/// re-entrant access inside the region fails like an access outside any
/// scope.
pub(crate) struct ViewBufferScope<'a> {
    slot: &'a mut Option<Box<ViewBuffer>>,
}

impl<'a> ViewBufferScope<'a> {
    fn swap(slot: &'a mut Option<Box<ViewBuffer>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        Self { slot }
    }

    /// Installs the buffer in `slot` for the lifetime of the returned guard.
    ///
    /// The slot must hold a buffer. Whatever an enclosing build had
    /// installed is parked in the slot meanwhile and restored when the
    /// guard drops, which also moves the installed buffer back into the
    /// slot.
    pub(crate) fn install(slot: &'a mut Option<Box<ViewBuffer>>) -> Self {
        debug_assert!(
            slot.is_some(),
            "a scope is installed from a slot holding a buffer"
        );
        Self::swap(slot)
    }

    /// Returns whether a scope is active on the current thread, meaning an
    /// enclosing build has its buffer installed.
    pub(crate) fn is_active() -> bool {
        let buffer = CURRENT.take();
        let active = buffer.is_some();
        CURRENT.set(buffer);
        active
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
    pub(crate) fn with<R>(f: impl FnOnce(&mut ViewBuffer) -> R) -> R {
        let mut slot = None;
        let scope = ViewBufferScope::swap(&mut slot);
        let buffer = scope.slot.as_deref_mut().unwrap_or_else(|| {
            panic!(
                "no view is building on the current task: build views with `view!`, \
                 on the task that polls the outermost invocation"
            )
        });
        f(buffer)
    }

    /// Appends one view's instruction block to the installed buffer in one
    /// synchronous burst, filled by `f` through a [`PartsWriter`].
    ///
    /// # Panics
    ///
    /// Panics if no scope is active on the current thread.
    pub(crate) fn block(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        Self::with(|buffer| buffer.block(f))
    }
}

impl Drop for ViewBufferScope<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;

    #[test]
    fn no_scope_is_active_by_default() {
        assert!(!ViewBufferScope::is_active());
    }

    #[test]
    fn install_makes_the_buffer_reachable_until_the_guard_drops() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        {
            let _scope = ViewBufferScope::install(&mut slot);
            assert!(ViewBufferScope::is_active());
            ViewBufferScope::block(|parts| {
                parts.push_str("a < b");
            });
        }
        assert!(!ViewBufferScope::is_active());
        let mut buffer = slot.expect("the buffer was swapped back on exit");
        let view = buffer.block(|parts| {
            parts.push_str("x");
        });
        assert_eq!(view.seal(*buffer).render(&Cx::default()), "x");
    }

    #[test]
    fn a_nested_install_parks_the_enclosing_buffer() {
        let mut outer = Some(Box::new(ViewBuffer::new()));
        let _outer_scope = ViewBufferScope::install(&mut outer);
        let outer_view = ViewBufferScope::block(|parts| {
            parts.push_str("outer");
        });

        let mut inner = Some(Box::new(ViewBuffer::new()));
        let inner_view = {
            let _inner_scope = ViewBufferScope::install(&mut inner);
            ViewBufferScope::block(|parts| {
                parts.push_str("inner");
            })
        };
        let inner_view = inner_view.seal(*inner.expect("the inner buffer was swapped back"));

        // The outer buffer is installed again, so the inner view splices in.
        let view = ViewBufferScope::block(|parts| {
            parts.push_view_handle(outer_view);
            parts.push_view_handle(inner_view);
        });
        drop(_outer_scope);
        let view = view.seal(*outer.expect("the outer buffer was swapped back"));
        assert_eq!(view.render(&Cx::default()), "outerinner");
    }

    #[test]
    #[should_panic(expected = "no view is building on the current task")]
    fn with_panics_outside_a_scope() {
        ViewBufferScope::with(|_buffer| {});
    }

    #[test]
    #[should_panic(expected = "no view is building on the current task")]
    fn with_is_not_reentrant() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let _scope = ViewBufferScope::install(&mut slot);
        ViewBufferScope::with(|_outer| {
            ViewBufferScope::with(|_inner| {});
        });
    }

    #[test]
    fn the_guard_restores_the_buffer_when_the_region_panics() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = ViewBufferScope::install(&mut slot);
            panic!("boom");
        }));
        assert!(result.is_err());
        assert!(slot.is_some());
        assert!(!ViewBufferScope::is_active());
    }
}
