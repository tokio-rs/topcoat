use crate::render::{Instruction, ReadOnlyMemory};

pub struct Memory {
    instructions: Vec<Instruction>,
    rom: ReadOnlyMemory,
}

impl Memory {}
