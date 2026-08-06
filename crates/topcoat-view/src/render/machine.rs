use crate::{
    Formatter,
    render::{Instruction, InstructionPtr, Memory, ReadOnlyMemory},
};

pub struct Machine<'a> {
    memory: &'a Memory,
    ip: InstructionPtr,
    stack: Vec<InstructionPtr>,
}

impl<'a> Machine<'a> {
    pub fn new(memory: &'a Memory, ip: InstructionPtr) -> Self {
        Self {
            memory,
            ip,
            stack: Vec::new(),
        }
    }

    fn push(&mut self) {
        self.stack.push(self.ip);
    }

    fn pop(&mut self) {
        self.ip = self.stack.pop().expect("popped empty stack");
    }

    pub fn execute(&mut self, rom: &ReadOnlyMemory, f: &mut Formatter<'_>) {
        loop {
            let instruction = self.memory.instruction(self.ip);
            self.ip.increment();

            use std::fmt::Write;
            let mut int_buffer = itoa::Buffer::new();

            match instruction {
                Instruction::Call { ip: to } => {
                    self.push();
                    self.ip = to;
                }
                Instruction::Ret => {
                    if self.stack.is_empty() {
                        break;
                    }
                    self.pop();
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
                    context.writer(f).write_str(rom.fetch_static_str(*ptr))
                }
                Instruction::String { ptr, context } => {
                    context.writer(f).write_str(rom.fetch_string(*ptr))
                }

                #[cfg(feature = "http")]
                Instruction::StatusCode(status_code) => f.record_status_code(*status_code),
                #[cfg(feature = "http")]
                Instruction::Headers { ptr } => f.record_headers(rom.fetch_headers(*ptr).clone()),
            }
        }
    }
}
