mod const_buffer;
mod handle;
mod id;
mod instruction;
mod instruction_buffer;
mod part;
mod renderer;
mod scope;

use core::fmt::NumBuffer;

#[cfg(feature = "http")]
use const_buffer::HeadersPtr;
use const_buffer::{ConstBuffer, DynPtr, StaticStrPtr, StrPtr, StringPtr, ViewPtr};
pub use handle::*;
use id::ViewBufferId;
use instruction::Instruction;
use instruction_buffer::{InstructionBuffer, InstructionPtr};
pub use part::*;
use renderer::Renderer;
pub(crate) use scope::*;

use crate::{HtmlContext, RegionId};

/// The instruction buffer of a build.
///
/// The outermost view of a build creates a buffer and installs it in a
/// [`ViewBufferScope`] for the duration of each of its polls. Every view
/// built inside those polls appends its instructions here, and rendering a
/// [`ViewHandle`] executes them. Once the outermost view resolves its
/// content, the buffer is sealed into the handle, which then carries it
/// wherever it renders.
///
/// # Contiguity
///
/// A nested view is a `(buffer id, entry)` pair pointing into this shared,
/// append-only sequence, so the instructions of one view must form a
/// contiguous block terminated by a return instruction. Callers uphold
/// this by pushing a whole block in one synchronous burst: no `await` may
/// happen between a block's first push and its final return instruction.
/// Futures interleave only at await points, so concurrently built sibling
/// views each still land in one piece.
#[derive(Debug)]
pub(crate) struct ViewBuffer {
    id: ViewBufferId,
    instructions: InstructionBuffer,
    consts: ConstBuffer,
}

impl Default for ViewBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewBuffer {
    /// Creates an empty buffer.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            id: ViewBufferId::next(),
            instructions: InstructionBuffer::new(),
            consts: ConstBuffer::new(),
        }
    }

    /// Returns this buffer's unique id.
    #[inline]
    fn id(&self) -> ViewBufferId {
        self.id
    }

    /// Builds a self-contained view in one synchronous burst from the parts
    /// `f` pushes: one block in a fresh buffer, sealed into the handle.
    #[cfg(test)]
    pub(crate) fn build(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        let mut buffer = Self::new();
        buffer.block(f).seal(buffer)
    }

    /// Appends one view's instruction block in one synchronous burst,
    /// filled by `f` through a [`PartsWriter`].
    ///
    /// Records the entry address, runs `f`, and terminates the block with a
    /// return instruction. Returns the handle to the block, carrying the
    /// writer's accumulated size hint. `f` must not build other views in
    /// this buffer; nested views are built first and spliced into the block
    /// with [`PartsWriter::push_view_handle`].
    pub(crate) fn block(&mut self, f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        let entry = self.next_ptr();
        let mut parts = PartsWriter::new(self, HtmlContext::Text);
        f(&mut parts);
        let size_hint = parts.size_hint();
        self.push_ret();
        ViewHandle::from_scope(self.id, entry, size_hint)
    }

    /// Returns the address the next pushed instruction will live at.
    #[inline]
    fn next_ptr(&self) -> InstructionPtr {
        self.instructions.next_ptr()
    }

    #[inline]
    fn instruction(&self, ptr: InstructionPtr) -> &Instruction {
        self.instructions.fetch(ptr)
    }

    #[inline]
    fn consts(&self) -> &ConstBuffer {
        &self.consts
    }

    #[inline]
    fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Appends a nested view, such as a rendered component.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building buffer.
    #[inline]
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
    #[inline]
    fn push_ret(&mut self) {
        self.push_instruction(Instruction::Ret);
    }

    #[inline]
    fn push_bool(&mut self, value: bool) {
        self.push_instruction(Instruction::Bool(value));
    }

    #[inline]
    fn push_i8(&mut self, value: i8) {
        self.push_instruction(Instruction::I8(value));
    }

    #[inline]
    fn push_i16(&mut self, value: i16) {
        self.push_instruction(Instruction::I16(value));
    }

    #[inline]
    fn push_i32(&mut self, value: i32) {
        self.push_instruction(Instruction::I32(value));
    }

    #[inline]
    fn push_i64(&mut self, value: i64) {
        self.push_instruction(Instruction::I64(value));
    }

    #[inline]
    fn push_isize(&mut self, value: isize) {
        self.push_instruction(Instruction::Isize(value));
    }

    #[inline]
    fn push_u8(&mut self, value: u8) {
        self.push_instruction(Instruction::U8(value));
    }

    #[inline]
    fn push_u16(&mut self, value: u16) {
        self.push_instruction(Instruction::U16(value));
    }

    #[inline]
    fn push_u32(&mut self, value: u32) {
        self.push_instruction(Instruction::U32(value));
    }

    #[inline]
    fn push_u64(&mut self, value: u64) {
        self.push_instruction(Instruction::U64(value));
    }

    #[inline]
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

    #[inline]
    fn push_f32(&mut self, value: f32) {
        self.push_instruction(Instruction::F32(value));
    }

    #[inline]
    fn push_f64(&mut self, value: f64) {
        self.push_instruction(Instruction::F64(value));
    }

    #[inline]
    fn push_char(&mut self, value: char, context: HtmlContext) {
        self.push_instruction(Instruction::Char { value, context });
    }

    /// Appends a static string held by reference.
    ///
    /// Pass `&"..."`, which Rust promotes to a reference into the binary's
    /// read-only data. The string stays out of the buffer's constants, so
    /// prefer this over [`push_static_str`](Self::push_static_str) whenever
    /// the string is written as a literal.
    #[inline]
    fn push_promoted_str(&mut self, value: &'static &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        self.push_instruction(Instruction::PromotedStr { value, context });
    }

    #[inline]
    fn push_static_str(&mut self, value: &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.consts.push_static_str(value);
        self.push_instruction(Instruction::StaticStr { ptr, context });
    }

    #[inline]
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

    #[inline]
    fn push_string(&mut self, value: String, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let ptr = self.consts.push_string(value);
        self.push_instruction(Instruction::String { ptr, context });
    }

    #[inline]
    fn push_region_start(&mut self, value: RegionId) {
        self.push_instruction(Instruction::RegionStart(value));
    }

    #[inline]
    fn push_region_end(&mut self, value: RegionId) {
        self.push_instruction(Instruction::RegionEnd(value));
    }

    #[inline]
    fn push_dyn(&mut self, value: Box<dyn DynViewPart>, context: HtmlContext) {
        let ptr = self.consts.push_dyn(value);
        self.push_instruction(Instruction::Dyn { ptr, context });
    }

    #[cfg(feature = "http")]
    #[inline]
    fn push_status_code(&mut self, value: http::StatusCode) {
        self.push_instruction(Instruction::StatusCode(value));
    }

    #[cfg(feature = "http")]
    #[inline]
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
