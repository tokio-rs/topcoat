use std::collections::BTreeMap;

use crate::buffer::Instruction;

/// The address of an instruction in an [`InstructionBuffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionPtr(usize);

impl InstructionPtr {
    pub(crate) fn increment(&mut self) {
        self.0 += 1;
    }
}

/// The instructions of a [`ViewBuffer`](crate::buffer::ViewBuffer): an
/// append-only sequence addressed by [`InstructionPtr`].
#[derive(Debug)]
pub struct InstructionBuffer {
    instructions: Vec<Instruction>,
}

impl InstructionBuffer {
    pub(crate) fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    /// Returns the address the next pushed instruction will live at.
    #[must_use]
    pub fn next_ptr(&self) -> InstructionPtr {
        InstructionPtr(self.instructions.len())
    }

    pub(crate) fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub(crate) fn fetch(&self, ptr: InstructionPtr) -> &Instruction {
        &self.instructions[ptr.0]
    }

    pub(crate) fn fetch_mut(&mut self, ptr: InstructionPtr) -> &mut Instruction {
        &mut self.instructions[ptr.0]
    }

    /// Prints how many instructions of each kind the buffer holds.
    #[allow(unused)]
    pub(crate) fn print_stats(&self) {
        println!(
            "  instructions: {} ({} bytes)",
            self.instructions.len(),
            self.instructions.len() * std::mem::size_of::<Instruction>(),
        );
        let mut counts = BTreeMap::new();
        for instruction in &self.instructions {
            let name = match instruction {
                Instruction::Call { .. } => "Call",
                Instruction::Ret => "Ret",
                Instruction::Jmp { .. } => "Jmp",
                Instruction::Placeholder => "Placeholder",
                Instruction::View { .. } => "View",
                Instruction::Bool(_) => "Bool",
                Instruction::I8(_) => "I8",
                Instruction::I16(_) => "I16",
                Instruction::I32(_) => "I32",
                Instruction::I64(_) => "I64",
                Instruction::Isize(_) => "Isize",
                Instruction::U8(_) => "U8",
                Instruction::U16(_) => "U16",
                Instruction::U32(_) => "U32",
                Instruction::U64(_) => "U64",
                Instruction::Usize(_) => "Usize",
                Instruction::F32(_) => "F32",
                Instruction::F64(_) => "F64",
                Instruction::Char { .. } => "Char",
                Instruction::StaticStr { .. } => "StaticStr",
                Instruction::Str { .. } => "Str",
                Instruction::String { .. } => "String",
                Instruction::Dyn { .. } => "Dyn",
                #[cfg(feature = "http")]
                Instruction::StatusCode(_) => "StatusCode",
                #[cfg(feature = "http")]
                Instruction::Headers { .. } => "Headers",
            };
            *counts.entry(name).or_insert(0usize) += 1;
        }
        for (name, count) in counts {
            println!("    {name}: {count}");
        }
    }
}
