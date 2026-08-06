use crate::{
    HtmlContext,
    render::{Instruction, ReadOnlyMemory},
};

#[derive(Debug, Clone, Copy)]
pub struct InstructionPtr(usize);

impl InstructionPtr {
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

pub struct Memory {
    instructions: Vec<Instruction>,
    rom: ReadOnlyMemory,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            rom: ReadOnlyMemory::new(),
        }
    }

    pub fn instruction(&self, ip: InstructionPtr) -> &Instruction {
        &self.instructions[ip.0]
    }

    fn push_instruction(&mut self, instruction: Instruction) -> InstructionPtr {
        self.instructions.push(instruction);
        InstructionPtr(self.instructions.len() - 1)
    }

    pub fn push_bool(&mut self, value: bool) -> InstructionPtr {
        self.push_instruction(Instruction::Bool(value))
    }

    pub fn push_i8(&mut self, value: i8) -> InstructionPtr {
        self.push_instruction(Instruction::I8(value))
    }

    pub fn push_i16(&mut self, value: i16) -> InstructionPtr {
        self.push_instruction(Instruction::I16(value))
    }

    pub fn push_i32(&mut self, value: i32) -> InstructionPtr {
        self.push_instruction(Instruction::I32(value))
    }

    pub fn push_i64(&mut self, value: i64) -> InstructionPtr {
        self.push_instruction(Instruction::I64(value))
    }

    pub fn push_isize(&mut self, value: isize) -> InstructionPtr {
        self.push_instruction(Instruction::Isize(value))
    }

    pub fn push_u8(&mut self, value: u8) -> InstructionPtr {
        self.push_instruction(Instruction::U8(value))
    }

    pub fn push_u16(&mut self, value: u16) -> InstructionPtr {
        self.push_instruction(Instruction::U16(value))
    }

    pub fn push_u32(&mut self, value: u32) -> InstructionPtr {
        self.push_instruction(Instruction::U32(value))
    }

    pub fn push_u64(&mut self, value: u64) -> InstructionPtr {
        self.push_instruction(Instruction::U64(value))
    }

    pub fn push_usize(&mut self, value: usize) -> InstructionPtr {
        self.push_instruction(Instruction::Usize(value))
    }

    pub fn push_f32(&mut self, value: f32) -> InstructionPtr {
        self.push_instruction(Instruction::F32(value))
    }

    pub fn push_f64(&mut self, value: f64) -> InstructionPtr {
        self.push_instruction(Instruction::F64(value))
    }

    pub fn push_char(&mut self, value: char, context: HtmlContext) -> InstructionPtr {
        self.push_instruction(Instruction::Char { value, context })
    }

    pub fn push_static_str(&mut self, value: &'static str, context: HtmlContext) -> InstructionPtr {
        let ptr = self.rom.push_static_str(value);
        self.push_instruction(Instruction::StaticStr { ptr, context })
    }

    pub fn push_string(&mut self, value: String, context: HtmlContext) -> InstructionPtr {
        let ptr = self.rom.push_string(value);
        self.push_instruction(Instruction::String { ptr, context })
    }

    #[cfg(feature = "http")]
    pub fn push_status_code(&mut self, value: http::StatusCode) -> InstructionPtr {
        self.push_instruction(Instruction::StatusCode(value))
    }

    #[cfg(feature = "http")]
    pub fn push_headers(&mut self, value: http::HeaderMap) -> InstructionPtr {
        let ptr = self.rom.push_headers(value);
        self.push_instruction(Instruction::Headers { ptr })
    }
}
