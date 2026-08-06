use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    DynViewPart, HtmlContext, View,
    render::{ConstPool, Instruction},
    view::ViewRepr,
};

/// The address of an instruction in a [`Memory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionPtr(usize);

impl InstructionPtr {
    pub(crate) fn increment(&mut self) {
        self.0 += 1;
    }
}

/// The identity of a [`Memory`], unique for the lifetime of the process.
///
/// A [`View`] records the id of the memory its instructions live in, so
/// rendering it inside a different scope fails instead of executing another
/// scope's instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryId(u64);

impl MemoryId {
    fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// The instruction memory of a view scope.
///
/// Every `view!` invocation inside a [`scope`](crate::scope) appends its
/// instructions here, and rendering a [`View`] executes them.
///
/// # Contiguity
///
/// A view is a `(memory id, entry)` pair pointing into this shared, append-
/// only sequence, so the instructions of one view must form a contiguous,
/// [`Ret`](Instruction::Ret)-terminated block. Callers uphold this by pushing
/// a whole block in one synchronous burst: no `await` may happen between a
/// block's first push and its final [`push_ret`](Self::push_ret). Futures
/// interleave only at await points, so concurrently built sibling views each
/// still land in one piece.
#[derive(Debug)]
pub struct Memory {
    id: MemoryId,
    instructions: Vec<Instruction>,
    pool: ConstPool,
    size_hint: usize,
}

impl Memory {
    pub(crate) fn new() -> Self {
        Self {
            id: MemoryId::next(),
            instructions: Vec::new(),
            pool: ConstPool::new(),
            size_hint: 0,
        }
    }

    /// Returns this memory's unique id.
    #[must_use]
    pub fn id(&self) -> MemoryId {
        self.id
    }

    /// Returns the address the next pushed instruction will live at.
    #[must_use]
    pub fn next_ptr(&self) -> InstructionPtr {
        InstructionPtr(self.instructions.len())
    }

    pub(crate) fn instruction(&self, ptr: InstructionPtr) -> &Instruction {
        &self.instructions[ptr.0]
    }

    pub(crate) fn pool(&self) -> &ConstPool {
        &self.pool
    }

    /// Returns an estimate of the number of bytes rendering this memory's
    /// content will write.
    ///
    /// The estimate covers everything pushed so far, so it is an over-
    /// estimate for a view that spans only part of the memory. Falling short
    /// forces the output buffer to grow and copy, whereas an over-estimate
    /// only leaves some capacity unused, so the excess is acceptable.
    pub(crate) fn size_hint(&self) -> usize {
        self.size_hint
    }

    fn push_instruction(&mut self, instruction: Instruction, size_hint: usize) {
        self.instructions.push(instruction);
        self.size_hint += size_hint;
    }

    /// Appends a nested view, such as a rendered component.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different scope.
    pub fn push_view(&mut self, view: View) {
        match view.repr() {
            ViewRepr::Static(body) => {
                self.push_static_str(body, HtmlContext::Unescaped);
            }
            ViewRepr::Scoped { memory, entry } => {
                assert!(
                    memory == self.id,
                    "tried to use a view outside the scope it was built in",
                );
                self.push_instruction(Instruction::Call { entry }, 0);
            }
        }
    }

    /// Appends the return instruction that terminates a view's instruction
    /// block.
    pub fn push_ret(&mut self) {
        self.push_instruction(Instruction::Ret, 0);
    }

    // Each numeric size hint is the midpoint, rounded up, between the
    // shortest and widest output the type can render, including the leading
    // `-` for signed types (`isize`/`usize` assume a 64-bit target). A
    // float's rendered width is unbounded for extreme magnitudes, so the
    // upper end is the shortest round-trip form of a typical value.

    pub fn push_bool(&mut self, value: bool) {
        self.push_instruction(Instruction::Bool(value), 5);
    }

    pub fn push_i8(&mut self, value: i8) {
        self.push_instruction(Instruction::I8(value), 3);
    }

    pub fn push_i16(&mut self, value: i16) {
        self.push_instruction(Instruction::I16(value), 4);
    }

    pub fn push_i32(&mut self, value: i32) {
        self.push_instruction(Instruction::I32(value), 6);
    }

    pub fn push_i64(&mut self, value: i64) {
        self.push_instruction(Instruction::I64(value), 11);
    }

    pub fn push_isize(&mut self, value: isize) {
        self.push_instruction(Instruction::Isize(value), 11);
    }

    pub fn push_u8(&mut self, value: u8) {
        self.push_instruction(Instruction::U8(value), 2);
    }

    pub fn push_u16(&mut self, value: u16) {
        self.push_instruction(Instruction::U16(value), 3);
    }

    pub fn push_u32(&mut self, value: u32) {
        self.push_instruction(Instruction::U32(value), 6);
    }

    pub fn push_u64(&mut self, value: u64) {
        self.push_instruction(Instruction::U64(value), 11);
    }

    pub fn push_usize(&mut self, value: usize) {
        self.push_instruction(Instruction::Usize(value), 11);
    }

    /// Appends an `i128` rendered as text.
    ///
    /// An `i128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant pool. Its digits are not significant in
    /// any HTML context, so no escaping applies.
    pub fn push_i128(&mut self, value: i128) {
        let mut buffer = itoa::Buffer::new();
        self.push_string(buffer.format(value).to_owned(), HtmlContext::Unescaped);
    }

    /// Appends a `u128` rendered as text.
    ///
    /// A `u128` does not fit into a fixed-size instruction, so its rendered
    /// form is stored in the constant pool. Its digits are not significant in
    /// any HTML context, so no escaping applies.
    pub fn push_u128(&mut self, value: u128) {
        let mut buffer = itoa::Buffer::new();
        self.push_string(buffer.format(value).to_owned(), HtmlContext::Unescaped);
    }

    pub fn push_f32(&mut self, value: f32) {
        self.push_instruction(Instruction::F32(value), 9);
    }

    pub fn push_f64(&mut self, value: f64) {
        self.push_instruction(Instruction::F64(value), 13);
    }

    pub fn push_char(&mut self, value: char, context: HtmlContext) {
        // One to four UTF-8 bytes, or an escape sequence.
        self.push_instruction(Instruction::Char { value, context }, 3);
    }

    pub fn push_static_str(&mut self, value: &'static str, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let size_hint = Self::str_size_hint(value, context);
        let ptr = self.pool.push_static_str(value);
        self.push_instruction(Instruction::StaticStr { ptr, context }, size_hint);
    }

    pub fn push_string(&mut self, value: String, context: HtmlContext) {
        if value.is_empty() {
            return;
        }
        let size_hint = Self::str_size_hint(&value, context);
        let ptr = self.pool.push_string(value);
        self.push_instruction(Instruction::String { ptr, context }, size_hint);
    }

    pub fn push_dyn(&mut self, value: Box<dyn DynViewPart>, context: HtmlContext) {
        let size_hint = value.size_hint();
        let ptr = self.pool.push_dyn(value);
        self.push_instruction(Instruction::Dyn { ptr, context }, size_hint);
    }

    #[cfg(feature = "http")]
    pub fn push_status_code(&mut self, value: http::StatusCode) {
        self.push_instruction(Instruction::StatusCode(value), 0);
    }

    #[cfg(feature = "http")]
    pub fn push_headers(&mut self, value: http::HeaderMap) {
        let ptr = self.pool.push_headers(value);
        self.push_instruction(Instruction::Headers { ptr }, 0);
    }

    fn str_size_hint(value: &str, context: HtmlContext) -> usize {
        match context {
            HtmlContext::Unescaped => value.len(),
            // Assume some characters escape into multi-byte sequences.
            _ => value.len() + value.len() / 8,
        }
    }
}
