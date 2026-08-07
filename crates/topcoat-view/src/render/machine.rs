use topcoat_core::context::Cx;

use crate::{
    Formatter,
    render::{Instruction, InstructionPtr, Memory, StrPtr},
};

/// Executes a view's instruction block into a [`Formatter`].
///
/// Execution starts at the view's entry, descends into nested views through
/// [`Call`](Instruction::Call) instructions, follows the
/// [`Jmp`](Instruction::Jmp) redirects of filled view slots, and finishes
/// when a [`Ret`](Instruction::Ret) is reached with an empty call stack.
pub struct Machine<'a> {
    memory: &'a Memory,
    ptr: InstructionPtr,
    stack: Vec<InstructionPtr>,
}

impl<'a> Machine<'a> {
    #[must_use]
    pub fn new(memory: &'a Memory, entry: InstructionPtr) -> Self {
        Self {
            memory,
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

        let pool = self.memory.pool();
        let mut int_buffer = itoa::Buffer::new();
        loop {
            let instruction = self.memory.instruction(self.ptr);
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

                Instruction::Bool(inner) => f.write_str(if *inner { "true" } else { "false" }),
                Instruction::I8(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::I16(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::I32(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::I64(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::Isize(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::U8(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::U16(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::U32(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::U64(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::Usize(inner) => f.write_str(int_buffer.format(*inner)),
                Instruction::F32(inner) => write!(f, "{inner}").unwrap(),
                Instruction::F64(inner) => write!(f, "{inner}").unwrap(),
                Instruction::Char { value, context } => context.writer(f).write_char(*value),

                Instruction::StaticStr { ptr, context } => {
                    context.writer(f).write_str(pool.fetch_static_str(*ptr));
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
                    context.writer(f).write_str(pool.fetch_str(ptr));
                }
                Instruction::String { ptr, context } => {
                    context.writer(f).write_str(pool.fetch_string(*ptr));
                }
                Instruction::Dyn { ptr, context } => {
                    pool.fetch_dyn(*ptr).render(cx, &mut context.writer(f));
                }

                #[cfg(feature = "http")]
                Instruction::StatusCode(status_code) => f.record_status_code(*status_code),
                #[cfg(feature = "http")]
                Instruction::Headers { ptr } => f.record_headers(pool.fetch_headers(*ptr).clone()),
            }
        }
    }
}
