use std::cell::Cell;

use crate::{
    PartsWriter, ViewHandle,
    buffer::{InstructionPtr, ViewBuffer},
};

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

/// A block under construction in the installed buffer.
///
/// Opening takes the buffer out of the scope, like
/// [`ViewBufferScope::with`] does for the duration of its closure, and
/// holds it until the block is closed, so a block can be filled across a
/// region of straight-line code instead of a single closure. While the
/// buffer is out, nothing else can build into it, and a re-entrant access
/// fails like an access outside any scope; a region that needs to wait,
/// such as an `await`, suspends the block meanwhile and resumes it after.
/// Dropping the guard without closing the block, as a panic in the region
/// does, puts the buffer back.
pub(crate) struct OpenBlock {
    /// The buffer; `None` while the block is suspended.
    buffer: Option<Box<ViewBuffer>>,
    entry: InstructionPtr,
    /// The jump ending the appended part of a suspended block, patched to
    /// the next instruction when the block resumes.
    suspended: Option<InstructionPtr>,
}

impl OpenBlock {
    /// Starts a block in the installed buffer.
    ///
    /// # Panics
    ///
    /// Panics if no scope is active on the current thread.
    pub(crate) fn open() -> Self {
        let mut buffer = Self::take();
        let entry = buffer.open_block();
        Self {
            buffer: Some(buffer),
            entry,
            suspended: None,
        }
    }

    /// Takes the installed buffer out of the scope.
    fn take() -> Box<ViewBuffer> {
        CURRENT.take().unwrap_or_else(|| {
            panic!(
                "no view is building on the current task: build views with `view!`, \
                 on the task that polls the outermost invocation"
            )
        })
    }

    /// Returns the buffer the block is appended to.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended.
    #[inline]
    pub(crate) fn buffer(&mut self) -> &mut ViewBuffer {
        self.buffer
            .as_deref_mut()
            .expect("the block is suspended: resume it before appending to it")
    }

    /// Puts the buffer back into the scope until [`resume`](Self::resume),
    /// so other blocks may be built meanwhile.
    ///
    /// The instructions appended after resuming continue the block through
    /// a jump, so the block stays one sequence to render.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended already.
    pub(crate) fn suspend(&mut self) {
        assert!(self.suspended.is_none(), "the block is suspended already");
        let mut buffer = self
            .buffer
            .take()
            .expect("a block that is not suspended holds the buffer");
        self.suspended = Some(buffer.suspend_block());
        CURRENT.set(Some(buffer));
    }

    /// Takes the buffer back out of the scope and continues the block.
    ///
    /// # Panics
    ///
    /// Panics if the block is not suspended, or if no scope is active on
    /// the current thread.
    pub(crate) fn resume(&mut self) {
        let jmp = self.suspended.take().expect("the block is not suspended");
        let mut buffer = Self::take();
        buffer.resume_block(jmp);
        self.buffer = Some(buffer);
    }

    /// Terminates the block, puts the buffer back into the scope, and
    /// returns the handle to the block.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended.
    pub(crate) fn close(mut self) -> ViewHandle {
        let mut buffer = self
            .buffer
            .take()
            .expect("the block is suspended: resume it before closing it");
        let handle = buffer.close_block(self.entry);
        CURRENT.set(Some(buffer));
        handle
    }
}

impl Drop for OpenBlock {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            CURRENT.set(Some(buffer));
        }
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
        let outer_scope = ViewBufferScope::install(&mut outer);
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
        drop(outer_scope);
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
    fn an_open_block_holds_the_buffer_until_it_is_closed() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let view = {
            let _scope = ViewBufferScope::install(&mut slot);
            let mut block = OpenBlock::open();
            assert!(!ViewBufferScope::is_active());
            PartsWriter::new(block.buffer(), crate::HtmlContext::Text).push_str("a < b");
            let view = block.close();
            assert!(ViewBufferScope::is_active());
            view
        };
        let buffer = slot.expect("the buffer was swapped back on exit");
        assert_eq!(view.seal(*buffer).render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn a_suspended_block_resumes_after_blocks_built_in_between() {
        let cx = &Cx::default();
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let (view, other) = {
            let _scope = ViewBufferScope::install(&mut slot);
            let mut block = OpenBlock::open();
            PartsWriter::new(block.buffer(), crate::HtmlContext::Text).push_str("a");
            block.suspend();
            assert!(ViewBufferScope::is_active());
            let other = ViewBufferScope::block(|parts| {
                parts.push_str("other");
            });
            block.resume();
            assert!(!ViewBufferScope::is_active());
            PartsWriter::new(block.buffer(), crate::HtmlContext::Text).push_str("b");
            block.suspend();
            block.resume();
            PartsWriter::new(block.buffer(), crate::HtmlContext::Text).push_str("c");
            (block.close(), other)
        };
        let _scope = ViewBufferScope::install(&mut slot);
        assert_eq!(view.render(cx), "abc");
        assert_eq!(other.render(cx), "other");
    }

    #[test]
    #[should_panic(expected = "resume it before appending")]
    fn appending_to_a_suspended_block_panics() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let _scope = ViewBufferScope::install(&mut slot);
        let mut block = OpenBlock::open();
        block.suspend();
        let _ = block.buffer();
    }

    #[test]
    fn dropping_a_suspended_block_leaves_the_buffer_in_the_scope() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let _scope = ViewBufferScope::install(&mut slot);
        let mut block = OpenBlock::open();
        block.suspend();
        drop(block);
        assert!(ViewBufferScope::is_active());
    }

    #[test]
    fn an_open_block_puts_the_buffer_back_when_the_region_panics() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let _scope = ViewBufferScope::install(&mut slot);
        let result = std::panic::catch_unwind(|| {
            let _block = OpenBlock::open();
            panic!("boom");
        });
        assert!(result.is_err());
        assert!(ViewBufferScope::is_active());
    }

    #[test]
    #[should_panic(expected = "no view is building on the current task")]
    fn opening_a_block_outside_a_scope_panics() {
        let _block = OpenBlock::open();
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
