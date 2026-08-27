use crate::buffer::{InstructionPtr, ViewBufferId, ViewBufferScope, ViewHandle};

/// A node position reserved in a block for a view that resolves later.
///
/// Reserving pushes a placeholder instruction into the block being built
/// and hands back the slot; [`fill`](Self::fill) overwrites the placeholder
/// with the resolved view once it is built. A block renders only after
/// every slot in it was filled.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewSlot {
    buffer: ViewBufferId,
    ptr: InstructionPtr,
}

impl ViewSlot {
    pub(super) fn new(buffer: ViewBufferId, ptr: InstructionPtr) -> Self {
        Self { buffer, ptr }
    }

    /// Returns the id of the buffer the slot was reserved in.
    pub(super) fn buffer(self) -> ViewBufferId {
        self.buffer
    }

    /// Returns the address of the slot's placeholder instruction.
    pub(super) fn ptr(self) -> InstructionPtr {
        self.ptr
    }

    /// Fills the slot with `view`, replacing its placeholder with exactly
    /// the instruction splicing the view would have pushed.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task, if the slot or
    /// the view belongs to a different buffer, or if the slot was filled
    /// already.
    pub(crate) fn fill(self, view: ViewHandle) {
        ViewBufferScope::with(|buffer| buffer.fill(self, view));
    }
}
