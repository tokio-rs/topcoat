mod const_buffer;
mod handle;
mod id;
mod instruction;
mod instruction_buffer;
mod part;
mod renderer;
mod scope;
mod view_slot;

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
pub(crate) use view_slot::*;

use crate::HtmlContext;

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
/// append-only sequence, so the instructions of one view must form one
/// sequence to execute from its entry to its return instruction. Callers
/// uphold this by pushing a whole block in one synchronous burst: no
/// `await` may happen between a block's first push and its final return
/// instruction. Futures interleave only at await points, so concurrently
/// built sibling views each still land in one piece. A block that must
/// wait suspends itself instead, ending its appended part with a jump that
/// continues it wherever it resumes, past whatever was appended meanwhile.
#[derive(Debug)]
pub(crate) struct ViewBuffer {
    id: ViewBufferId,
    instructions: InstructionBuffer,
    consts: ConstBuffer,
    /// An estimate of the number of bytes everything appended so far writes
    /// when rendered. Every part renders once, so the running total is the
    /// size hint of the content the buffer is sealed into.
    size_hint: usize,
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
            size_hint: 0,
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
    /// return instruction. Returns the handle to the block. `f` must not
    /// build other views in this buffer; nested views are built first and
    /// spliced into the block with [`PartsWriter::push_view_handle`], or
    /// resolved later into a slot reserved with [`PartsWriter::reserve`].
    pub(crate) fn block(&mut self, f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        let entry = self.open_block();
        f(&mut PartsWriter::new(self, HtmlContext::Text));
        self.close_block(entry)
    }

    /// Starts a block and returns its entry address.
    ///
    /// The instructions pushed until [`close_block`](Self::close_block) form
    /// the block.
    #[inline]
    pub(crate) fn open_block(&mut self) -> InstructionPtr {
        self.next_ptr()
    }

    /// Terminates the block started at `entry` with a return instruction
    /// and returns the handle to it.
    #[inline]
    pub(crate) fn close_block(&mut self, entry: InstructionPtr) -> ViewHandle {
        self.push_ret();
        ViewHandle::from_scope(self.id, entry)
    }

    /// Suspends the block being built: appends a jump whose target is
    /// decided when the block resumes, so other blocks may be appended in
    /// between, and returns the jump's address.
    #[inline]
    pub(crate) fn suspend_block(&mut self) -> InstructionPtr {
        let ptr = self.next_ptr();
        self.push_instruction(Instruction::Jmp { entry: ptr });
        ptr
    }

    /// Resumes a block suspended at `jmp`: the jump continues at the next
    /// instruction appended.
    #[inline]
    pub(crate) fn resume_block(&mut self, jmp: InstructionPtr) {
        let entry = self.next_ptr();
        *self.instructions.fetch_mut(jmp) = Instruction::Jmp { entry };
    }

    /// Returns the accumulated size hint of everything appended so far.
    #[inline]
    pub(super) fn size_hint(&self) -> usize {
        self.size_hint
    }

    /// Adds `bytes` to the accumulated size hint.
    #[inline]
    fn add_size_hint(&mut self, bytes: usize) {
        self.size_hint += bytes;
    }

    /// Reserves a node position in the block being built for a view that
    /// resolves later.
    ///
    /// Pushes a placeholder instruction and returns the slot to fill it
    /// through.
    #[inline]
    fn reserve(&mut self) -> ViewSlot {
        let ptr = self.next_ptr();
        self.push_instruction(Instruction::Placeholder);
        ViewSlot::new(self.id, ptr)
    }

    /// Fills a reserved slot with `view`, replacing its placeholder with
    /// exactly the instruction [`push_view`](Self::push_view) would have
    /// appended for the view.
    ///
    /// # Panics
    ///
    /// Panics if the slot or the view belongs to a different buffer, or if
    /// the slot was filled already.
    fn fill(&mut self, slot: ViewSlot, view: ViewHandle) {
        assert!(
            slot.buffer() == self.id,
            "tried to fill a view slot outside the `view!` invocation it was reserved in",
        );
        let instruction = if view.is_empty() {
            Instruction::PromotedStr {
                value: &"",
                context: HtmlContext::Unescaped,
            }
        } else {
            self.view_instruction(view)
        };
        let target = self.instructions.fetch_mut(slot.ptr());
        assert!(
            matches!(target, Instruction::Placeholder),
            "tried to fill a view slot twice",
        );
        *target = instruction;
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
        if view.is_empty() {
            return;
        }
        let instruction = self.view_instruction(view);
        self.push_instruction(instruction);
    }

    /// Returns the instruction splicing `view` into this buffer, recording
    /// the view's constants and size hint.
    ///
    /// A nested handle into this buffer becomes a call into its block; a
    /// static or owned handle is held in the constants.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building buffer.
    fn view_instruction(&mut self, view: ViewHandle) -> Instruction {
        self.add_size_hint(view.size_hint());
        match view.repr() {
            ViewRepr::Static(body) => Instruction::StaticStr {
                ptr: self.consts.push_static_str(body),
                context: HtmlContext::Unescaped,
            },
            ViewRepr::Scoped { buffer, entry } => {
                assert!(
                    buffer == self.id,
                    "tried to use a view outside the `view!` invocation it was built in",
                );
                Instruction::Call { entry }
            }
            ViewRepr::Owned { buffer, entry, .. } => Instruction::ViewHandle {
                ptr: self.consts.push_view(buffer, entry),
            },
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
