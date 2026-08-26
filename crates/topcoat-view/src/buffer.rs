mod const_buffer;
mod handle;
mod id;
mod instruction;
mod instruction_buffer;
mod part;
mod renderer;
mod view_slot;

use core::fmt::NumBuffer;

use const_buffer::*;
pub use handle::*;
use id::*;
use instruction::*;
use instruction_buffer::*;
pub use part::*;
use renderer::*;
use topcoat_core::context::Cx;
pub use view_slot::*;

use crate::{HtmlContext, internal::Builder};

/// The instruction buffer of a build.
///
/// The outermost `view!` invocation creates a buffer, every `view!`
/// invocation nested inside it appends its instructions here, and rendering
/// a [`ViewHandle`] executes them.
///
/// # Contiguity
///
/// A view is a `(buffer id, entry)` pair pointing into this shared, append-
/// only sequence, so the instructions of one view must form a contiguous,
/// [`Ret`](Instruction::Ret)-terminated block. Callers uphold this by pushing
/// a whole block in one synchronous burst: no `await` may happen between a
/// block's first push and its final [`push_ret`](Self::push_ret). Futures
/// interleave only at await points, so concurrently built sibling views each
/// still land in one piece.
#[derive(Debug)]
pub struct ViewBuffer {
    id: ViewBufferId,
    instructions: InstructionBuffer,
    consts: ConstBuffer,
}

impl ViewBuffer {
    pub(crate) fn new() -> Self {
        Self {
            id: ViewBufferId::next(),
            instructions: InstructionBuffer::new(),
            consts: ConstBuffer::new(),
        }
    }

    /// Returns this buffer's unique id.
    fn id(&self) -> ViewBufferId {
        self.id
    }

    /// Returns the address the next pushed instruction will live at.
    fn next_ptr(&self) -> InstructionPtr {
        self.instructions.next_ptr()
    }

    fn instruction(&self, ptr: InstructionPtr) -> &Instruction {
        self.instructions.fetch(ptr)
    }

    fn consts(&self) -> &ConstBuffer {
        &self.consts
    }

    fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Appends one view's instruction block in one synchronous burst,
    /// filled by `f` through a [`Builder`].
    ///
    /// Records the entry address, runs `f`, and terminates the block with a
    /// return instruction. Returns the handle to the block, carrying the
    /// builder's accumulated size hint. `f` must not build other views;
    /// nested views are built first and spliced into the block with
    /// [`Builder::view`].
    pub(crate) fn block(
        &mut self,
        cx: &Cx,
        f: impl FnOnce(&mut Builder<'_, '_, '_>),
    ) -> ViewHandle {
        self.write_block(|parts| f(&mut Builder::new(cx, parts)))
    }

    /// Appends one view's instruction block in one synchronous burst,
    /// filled by `f` through the writer directly.
    ///
    /// The writer counterpart of [`block`](Self::block), for compositions
    /// that push through the writer instead of a [`Builder`], like the
    /// runtime's JavaScript views.
    pub(crate) fn write_block(&mut self, f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        let entry = self.next_ptr();
        let mut parts = PartsWriter::new(self, HtmlContext::Text);
        f(&mut parts);
        let size_hint = parts.size_hint();
        self.push_ret();
        ViewHandle::from_scope(self.id, entry, size_hint)
    }

    /// Appends a nested view, such as a rendered component.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building buffer.
    fn push_view(&mut self, view: ViewHandle) {
        match view.repr() {
            ViewRepr::Static(body) => {
                self.push_static_str(body, HtmlContext::Unescaped);
            }
            ViewRepr::Scoped { buffer, entry, .. } => {
                assert!(
                    buffer == self.id,
                    "tried to use a view outside the `view!` invocation it was built in",
                );
                self.push_instruction(Instruction::Call { entry });
            }
            ViewRepr::Owned { buffer, entry, .. } => {
                let ptr = self.consts.push_view(buffer, entry);
                self.push_instruction(Instruction::ViewHandle { ptr });
            }
        }
    }

    /// Appends the return instruction that terminates a view's instruction
    /// block.
    fn push_ret(&mut self) {
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
    pub fn reserve_view(&mut self) -> (ViewHandle, ViewSlot) {
        let ptr = self.next_ptr();
        self.push_instruction(Instruction::Placeholder);
        let slot = ViewSlot::new(self.id, ptr);
        (ViewHandle::from_scope(self.id, ptr, 0), slot)
    }

    /// Redirects a reserved slot to `view`, resolving its placeholder.
    ///
    /// # Panics
    ///
    /// Panics if the slot was reserved in a different buffer, if the view
    /// was built in a different, still building buffer, or if the slot was
    /// already filled.
    pub fn fill_view(&mut self, slot: ViewSlot, view: ViewHandle) {
        assert!(
            slot.buffer() == self.id,
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
            ViewRepr::Scoped { buffer, entry, .. } => {
                assert!(
                    buffer == self.id,
                    "tried to use a view outside the `view!` invocation it was built in",
                );
                entry
            }
            // An owned view's block lives in its own buffer, so it is
            // materialized as a block holding one splice instruction.
            ViewRepr::Owned { buffer, entry, .. } => {
                let block_entry = self.next_ptr();
                let ptr = self.consts.push_view(buffer, entry);
                self.push_instruction(Instruction::ViewHandle { ptr });
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

    fn push_bool(&mut self, value: bool) {
        self.push_instruction(Instruction::Bool(value));
    }

    fn push_i8(&mut self, value: i8) {
        self.push_instruction(Instruction::I8(value));
    }

    fn push_i16(&mut self, value: i16) {
        self.push_instruction(Instruction::I16(value));
    }

    fn push_i32(&mut self, value: i32) {
        self.push_instruction(Instruction::I32(value));
    }

    fn push_i64(&mut self, value: i64) {
        self.push_instruction(Instruction::I64(value));
    }

    fn push_isize(&mut self, value: isize) {
        self.push_instruction(Instruction::Isize(value));
    }

    fn push_u8(&mut self, value: u8) {
        self.push_instruction(Instruction::U8(value));
    }

    fn push_u16(&mut self, value: u16) {
        self.push_instruction(Instruction::U16(value));
    }

    fn push_u32(&mut self, value: u32) {
        self.push_instruction(Instruction::U32(value));
    }

    fn push_u64(&mut self, value: u64) {
        self.push_instruction(Instruction::U64(value));
    }

    fn push_usize(&mut self, value: usize) {
        self.push_instruction(Instruction::Usize(value));
    }

    /// Appends an `i128` rendered as text.
    ///
    /// An `i128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant buffer. Its digits are not significant
    /// in any HTML context, so no escaping applies.
    fn push_i128(&mut self, value: i128) {
        self.push_str(
            value.format_into(&mut NumBuffer::new()),
            HtmlContext::Unescaped,
        );
    }

    /// Appends a `u128` rendered as text.
    ///
    /// A `u128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant buffer. Its digits are not significant
    /// in any HTML context, so no escaping applies.
    fn push_u128(&mut self, value: u128) {
        self.push_str(
            value.format_into(&mut NumBuffer::new()),
            HtmlContext::Unescaped,
        );
    }

    fn push_f32(&mut self, value: f32) {
        self.push_instruction(Instruction::F32(value));
    }

    fn push_f64(&mut self, value: f64) {
        self.push_instruction(Instruction::F64(value));
    }

    fn push_char(&mut self, value: char, context: HtmlContext) {
        self.push_instruction(Instruction::Char { value, context });
    }

    /// Appends a static string held by reference.
    ///
    /// Pass `&"..."`, which Rust promotes to a reference into the binary's
    /// read-only data. The string stays out of the buffer's constants, so
    /// prefer this over [`push_static_str`](Self::push_static_str) whenever
    /// the string is written as a literal.
    fn push_promoted_str(&mut self, value: &'static &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        self.push_instruction(Instruction::PromotedStr { value, context });
    }

    fn push_static_str(&mut self, value: &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.consts.push_static_str(value);
        self.push_instruction(Instruction::StaticStr { ptr, context });
    }

    fn push_str(&mut self, value: &str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let StrPtr { offset, len } = self.consts.push_str(value);
        self.push_instruction(Instruction::Str {
            offset,
            len,
            context,
        });
    }

    fn push_string(&mut self, value: String, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.consts.push_string(value);
        self.push_instruction(Instruction::String { ptr, context });
    }

    fn push_dyn(&mut self, value: Box<dyn DynViewPart>, context: HtmlContext) {
        let ptr = self.consts.push_dyn(value);
        self.push_instruction(Instruction::Dyn { ptr, context });
    }

    #[cfg(feature = "http")]
    fn push_status_code(&mut self, value: http::StatusCode) {
        self.push_instruction(Instruction::StatusCode(value));
    }

    #[cfg(feature = "http")]
    fn push_headers(&mut self, value: http::HeaderMap) {
        let ptr = self.consts.push_headers(value);
        self.push_instruction(Instruction::Headers { ptr });
    }

    /// Prints the buffer's fields and how many instructions and constants of
    /// each kind it holds.
    #[allow(unused)]
    fn print_stats(&self) {
        println!("ViewBuffer {{");
        println!("  id: {:?}", self.id);
        self.instructions.print_stats();
        self.consts.print_stats();
        println!("}}");
    }
}
