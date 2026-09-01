use std::collections::BTreeMap;

use crate::buffer::Instruction;

/// The address of an instruction in an [`InstructionBuffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InstructionPtr(usize);

impl InstructionPtr {
    #[inline]
    pub(super) fn increment(&mut self) {
        self.0 += 1;
    }
}

/// The instructions of a [`ViewBuffer`](crate::buffer::ViewBuffer): an
/// append-only sequence addressed by [`InstructionPtr`].
#[derive(Debug)]
pub(super) struct InstructionBuffer {
    instructions: Vec<Instruction>,
}

impl InstructionBuffer {
    pub(super) fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    /// Returns the address the next pushed instruction will live at.
    #[inline]
    #[must_use]
    pub(super) fn next_ptr(&self) -> InstructionPtr {
        InstructionPtr(self.instructions.len())
    }

    #[inline]
    pub(super) fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    #[inline]
    pub(super) fn fetch(&self, ptr: InstructionPtr) -> &Instruction {
        &self.instructions[ptr.0]
    }

    /// Prints how many instructions of each kind the buffer holds.
    #[allow(unused)]
    pub(super) fn print_stats(&self) {
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
                Instruction::ViewHandle { .. } => "ViewHandle",
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
                Instruction::PromotedStr { .. } => "PromotedStr",
                Instruction::StaticStr { .. } => "StaticStr",
                Instruction::Str { .. } => "Str",
                Instruction::String { .. } => "String",
                Instruction::Dyn { .. } => "Dyn",
                Instruction::RegionStart(_) => "RegionStart",
                Instruction::RegionEnd(_) => "RegionEnd",
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
