mod const_pool;
mod id;
mod instruction;
mod instruction_pool;
mod renderer;
mod scope;
mod view_slot;

pub use const_pool::*;
pub use id::*;
pub use instruction::*;
pub use instruction_pool::*;
pub use renderer::*;
pub use scope::*;
pub use view_slot::*;

use crate::{DynViewPart, HtmlContext, View, view::ViewRepr};

/// The instruction arena of a build.
///
/// The outermost `view!` invocation creates an arena, every `view!`
/// invocation nested inside it appends its instructions here, and rendering
/// a [`View`] executes them.
///
/// # Contiguity
///
/// A view is a `(arena id, entry)` pair pointing into this shared, append-
/// only sequence, so the instructions of one view must form a contiguous,
/// [`Ret`](Instruction::Ret)-terminated block. Callers uphold this by pushing
/// a whole block in one synchronous burst: no `await` may happen between a
/// block's first push and its final [`push_ret`](Self::push_ret). Futures
/// interleave only at await points, so concurrently built sibling views each
/// still land in one piece.
#[derive(Debug)]
pub struct Arena {
    id: ArenaId,
    instructions: InstructionPool,
    pool: ConstPool,
}

impl Arena {
    pub(crate) fn new() -> Self {
        Self {
            id: ArenaId::next(),
            instructions: InstructionPool::new(),
            pool: ConstPool::new(),
        }
    }

    /// Returns this arena's unique id.
    #[must_use]
    pub fn id(&self) -> ArenaId {
        self.id
    }

    /// Returns the address the next pushed instruction will live at.
    #[must_use]
    pub fn next_ptr(&self) -> InstructionPtr {
        self.instructions.next_ptr()
    }

    pub(crate) fn instruction(&self, ptr: InstructionPtr) -> &Instruction {
        self.instructions.fetch(ptr)
    }

    pub(crate) fn pool(&self) -> &ConstPool {
        &self.pool
    }

    fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Appends a nested view, such as a rendered component.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building arena.
    pub fn push_view(&mut self, view: View) {
        match view.repr() {
            ViewRepr::Static(body) => {
                self.push_static_str(body, HtmlContext::Unescaped);
            }
            ViewRepr::Scoped { arena, entry, .. } => {
                assert!(
                    arena == self.id,
                    "tried to use a view outside the `view!` invocation it was built in",
                );
                self.push_instruction(Instruction::Call { entry });
            }
            ViewRepr::Owned { arena, entry, .. } => {
                let ptr = self.pool.push_view(arena, entry);
                self.push_instruction(Instruction::View { ptr });
            }
        }
    }

    /// Appends the return instruction that terminates a view's instruction
    /// block.
    pub fn push_ret(&mut self) {
        self.push_instruction(Instruction::Ret);
    }

    /// Reserves a slot for a view that resolves later, such as the child of
    /// a concurrently rendering component.
    ///
    /// Returns a placeholder view pointing at the slot and the slot itself.
    /// Once [`fill_view`](Self::fill_view) redirects the slot, the
    /// placeholder renders the filled view's content; rendering it before
    /// that panics. The placeholder carries no size hint, since the filled
    /// view's is not known yet.
    pub fn reserve_view(&mut self) -> (View, ViewSlot) {
        let ptr = self.next_ptr();
        self.push_instruction(Instruction::Placeholder);
        let slot = ViewSlot::new(self.id, ptr);
        (View::from_scope(self.id, ptr, 0), slot)
    }

    /// Redirects a reserved slot to `view`, resolving its placeholder.
    ///
    /// # Panics
    ///
    /// Panics if the slot was reserved in a different arena, if the view
    /// was built in a different, still building arena, or if the slot was
    /// already filled.
    pub fn fill_view(&mut self, slot: ViewSlot, view: View) {
        assert!(
            slot.arena() == self.id,
            "tried to fill a view slot outside the `view!` invocation it was reserved in",
        );
        let entry = match view.repr() {
            // A static view has no block to jump to, so it is materialized
            // as one.
            ViewRepr::Static(body) => {
                let entry = self.next_ptr();
                self.push_static_str(body, HtmlContext::Unescaped);
                self.push_ret();
                entry
            }
            ViewRepr::Scoped { arena, entry, .. } => {
                assert!(
                    arena == self.id,
                    "tried to use a view outside the `view!` invocation it was built in",
                );
                entry
            }
            // An owned view's block lives in its own arena, so it is
            // materialized as a block holding one splice instruction.
            ViewRepr::Owned { arena, entry, .. } => {
                let block_entry = self.next_ptr();
                let ptr = self.pool.push_view(arena, entry);
                self.push_instruction(Instruction::View { ptr });
                self.push_ret();
                block_entry
            }
        };
        let instruction = self.instructions.fetch_mut(slot.ptr());
        assert!(
            matches!(instruction, Instruction::Placeholder),
            "tried to fill a view slot twice",
        );
        *instruction = Instruction::Jmp { entry };
    }

    pub fn push_bool(&mut self, value: bool) {
        self.push_instruction(Instruction::Bool(value));
    }

    pub fn push_i8(&mut self, value: i8) {
        self.push_instruction(Instruction::I8(value));
    }

    pub fn push_i16(&mut self, value: i16) {
        self.push_instruction(Instruction::I16(value));
    }

    pub fn push_i32(&mut self, value: i32) {
        self.push_instruction(Instruction::I32(value));
    }

    pub fn push_i64(&mut self, value: i64) {
        self.push_instruction(Instruction::I64(value));
    }

    pub fn push_isize(&mut self, value: isize) {
        self.push_instruction(Instruction::Isize(value));
    }

    pub fn push_u8(&mut self, value: u8) {
        self.push_instruction(Instruction::U8(value));
    }

    pub fn push_u16(&mut self, value: u16) {
        self.push_instruction(Instruction::U16(value));
    }

    pub fn push_u32(&mut self, value: u32) {
        self.push_instruction(Instruction::U32(value));
    }

    pub fn push_u64(&mut self, value: u64) {
        self.push_instruction(Instruction::U64(value));
    }

    pub fn push_usize(&mut self, value: usize) {
        self.push_instruction(Instruction::Usize(value));
    }

    /// Appends an `i128` rendered as text.
    ///
    /// An `i128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant pool. Its digits are not significant in
    /// any HTML context, so no escaping applies.
    pub fn push_i128(&mut self, value: i128) {
        let mut buffer = itoa::Buffer::new();
        self.push_str(buffer.format(value), HtmlContext::Unescaped);
    }

    /// Appends a `u128` rendered as text.
    ///
    /// A `u128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant pool. Its digits are not significant in
    /// any HTML context, so no escaping applies.
    pub fn push_u128(&mut self, value: u128) {
        let mut buffer = itoa::Buffer::new();
        self.push_str(buffer.format(value), HtmlContext::Unescaped);
    }

    pub fn push_f32(&mut self, value: f32) {
        self.push_instruction(Instruction::F32(value));
    }

    pub fn push_f64(&mut self, value: f64) {
        self.push_instruction(Instruction::F64(value));
    }

    pub fn push_char(&mut self, value: char, context: HtmlContext) {
        self.push_instruction(Instruction::Char { value, context });
    }

    pub fn push_static_str(&mut self, value: &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.pool.push_static_str(value);
        self.push_instruction(Instruction::StaticStr { ptr, context });
    }

    pub fn push_str(&mut self, value: &str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let StrPtr { offset, len } = self.pool.push_str(value);
        self.push_instruction(Instruction::Str {
            offset,
            len,
            context,
        });
    }

    pub fn push_string(&mut self, value: String, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.pool.push_string(value);
        self.push_instruction(Instruction::String { ptr, context });
    }

    pub fn push_dyn(&mut self, value: Box<dyn DynViewPart>, context: HtmlContext) {
        let ptr = self.pool.push_dyn(value);
        self.push_instruction(Instruction::Dyn { ptr, context });
    }

    #[cfg(feature = "http")]
    pub fn push_status_code(&mut self, value: http::StatusCode) {
        self.push_instruction(Instruction::StatusCode(value));
    }

    #[cfg(feature = "http")]
    pub fn push_headers(&mut self, value: http::HeaderMap) {
        let ptr = self.pool.push_headers(value);
        self.push_instruction(Instruction::Headers { ptr });
    }

    /// Prints the arena's fields and how many instructions and constants of
    /// each kind it holds.
    #[allow(unused)]
    pub(crate) fn print_stats(&self) {
        println!("Arena {{");
        println!("  id: {:?}", self.id);
        self.instructions.print_stats();
        self.pool.print_stats();
        println!("}}");
    }
}
