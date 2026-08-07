use crate::{
    View,
    arena::{ArenaId, ArenaScope, InstructionPtr},
};

/// A reserved slot in an arena that a view resolves into later.
///
/// Reserving pushes a placeholder and returns it as a view alongside the
/// slot; [`fill`](Self::fill) redirects the slot once the real view is
/// built.
#[derive(Debug, Clone, Copy)]
pub struct ViewSlot {
    arena: ArenaId,
    ptr: InstructionPtr,
}

impl ViewSlot {
    pub(crate) fn new(arena: ArenaId, ptr: InstructionPtr) -> Self {
        Self { arena, ptr }
    }

    /// Returns the id of the arena the slot was reserved in.
    pub(crate) fn arena(&self) -> ArenaId {
        self.arena
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
    /// view belongs to a different arena, or if the slot was already filled.
    pub fn fill(self, view: View) {
        ArenaScope::with(|arena| arena.fill_view(self, view));
    }
}
