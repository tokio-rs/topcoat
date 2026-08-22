use core::fmt::NumBuffer;

use topcoat_core::context::Cx;

use crate::{
    Formatter,
    buffer::{Instruction, InstructionPtr, StrPtr, ViewBuffer},
};

/// Executes a view's instruction block into a [`Formatter`].
///
/// Execution starts at the view's entry, descends into nested views through
/// [`Call`](Instruction::Call) instructions, follows the
/// [`Jmp`](Instruction::Jmp) redirects of filled view slots, and finishes
/// when a [`Ret`](Instruction::Ret) is reached with an empty call stack.
pub struct Renderer<'a> {
    buffer: &'a ViewBuffer,
    ptr: InstructionPtr,
    stack: Vec<InstructionPtr>,
}

impl<'a> Renderer<'a> {
    #[must_use]
    pub fn new(buffer: &'a ViewBuffer, entry: InstructionPtr) -> Self {
        Self {
            buffer,
            ptr: entry,
            stack: Vec::new(),
        }
    }

    /// Executes instructions from the entry until the block returns.
    ///
    /// # Panics
    ///
    /// Panics if execution reaches a reserved view slot that was never
    /// filled.
    pub fn execute(&mut self, cx: &Cx, f: &mut Formatter<'_>) {
        use std::fmt::Write;

        let consts = self.buffer.consts();
        loop {
            let instruction = self.buffer.instruction(self.ptr);
            self.ptr.increment();

            match instruction {
                Instruction::Call { entry } => {
                    self.stack.push(self.ptr);
                    self.ptr = *entry;
                }
                Instruction::Ret => match self.stack.pop() {
                    Some(ptr) => self.ptr = ptr,
                    None => break,
                },
                Instruction::Jmp { entry } => self.ptr = *entry,
                Instruction::Placeholder => {
                    panic!("tried to render a placeholder view before it was filled")
                }
                Instruction::View { ptr } => {
                    let (buffer, entry) = consts.fetch_view(*ptr);
                    Renderer::new(buffer, entry).execute(cx, f);
                }

                Instruction::Bool(inner) => f.write_str(if *inner { "true" } else { "false" }),
                Instruction::I8(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::I16(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::I32(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::I64(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::Isize(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::U8(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::U16(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::U32(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::U64(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::Usize(inner) => f.write_str(inner.format_into(&mut NumBuffer::new())),
                Instruction::F32(inner) => write!(f, "{inner}").unwrap(),
                Instruction::F64(inner) => write!(f, "{inner}").unwrap(),
                Instruction::Char { value, context } => context.writer(f).write_char(*value),

                Instruction::PromotedStr { value, context } => {
                    context.writer(f).write_str(value);
                }
                Instruction::StaticStr { ptr, context } => {
                    context.writer(f).write_str(consts.fetch_static_str(*ptr));
                }
                Instruction::Str {
                    offset,
                    len,
                    context,
                } => {
                    let ptr = StrPtr {
                        offset: *offset,
                        len: *len,
                    };
                    context.writer(f).write_str(consts.fetch_str(ptr));
                }
                Instruction::String { ptr, context } => {
                    context.writer(f).write_str(consts.fetch_string(*ptr));
                }
                Instruction::Dyn { ptr, context } => {
                    consts.fetch_dyn(*ptr).render(cx, &mut context.writer(f));
                }

                #[cfg(feature = "http")]
                Instruction::StatusCode(status_code) => f.record_status_code(*status_code),
                #[cfg(feature = "http")]
                Instruction::Headers { ptr } => {
                    f.record_headers(consts.fetch_headers(*ptr).clone());
                }
            }
        }
    }
}
