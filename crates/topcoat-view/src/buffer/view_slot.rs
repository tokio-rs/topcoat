use crate::{
    View,
    buffer::{InstructionPtr, ViewBufferId, ViewBufferScope},
};

/// A reserved slot in a view buffer that a view resolves into later.
///
/// Reserving pushes a placeholder and returns it as a view alongside the
/// slot; [`fill`](Self::fill) redirects the slot once the real view is
/// built.
#[derive(Debug, Clone, Copy)]
pub struct ViewSlot {
    buffer: ViewBufferId,
    ptr: InstructionPtr,
}

impl ViewSlot {
    pub(crate) fn new(buffer: ViewBufferId, ptr: InstructionPtr) -> Self {
        Self { buffer, ptr }
    }

    /// Returns the id of the buffer the slot was reserved in.
    pub(crate) fn buffer(&self) -> ViewBufferId {
        self.buffer
    }

    /// Returns the address of the slot's placeholder instruction.
    pub(crate) fn ptr(&self) -> InstructionPtr {
        self.ptr
    }

    /// Redirects this slot to `view`, resolving its placeholder.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task, if the slot or the
    /// view belongs to a different buffer, or if the slot was already filled.
    pub fn fill(self, view: View) {
        ViewBufferScope::with(|buffer| buffer.fill_view(self, view));
    }

    /// Redirects this slot to `view`, replacing whatever it held, and marks
    /// the buffer dirty so the driver sends a chunk.
    ///
    /// The live counterpart of [`fill`](Self::fill): the first fill of a
    /// reactive node's slot and every swap after it go through here.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task, or if the slot or
    /// the view belongs to a different buffer.
    pub fn refill(self, view: View) {
        ViewBufferScope::with(|buffer| buffer.refill_view(self, view));
    }
}
