use crate::buffer::{InstructionPtr, ViewBufferId};

/// A reserved slot in a view buffer that a view resolves into later.
///
/// Reserving pushes a placeholder and returns it as a view alongside the
/// slot; [`ViewBuffer::fill_view`](super::ViewBuffer::fill_view) redirects
/// the slot once the real view is built.
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
}
