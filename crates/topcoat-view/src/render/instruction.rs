use crate::{Formatter, HtmlContext};

pub struct InstructionMemory {
    instructions: Vec<Instruction>,
    rom: ReadOnlyMemory,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    /// Jump to a different point in the instruction memory.
    Call { ip: usize },
    /// Return back to the previous call instruction, if any.
    Ret,

    /// A boolean rendered as text.
    #[non_exhaustive]
    Bool(bool),
    /// An `i8` rendered as text.
    #[non_exhaustive]
    I8(i8),
    /// An `i16` rendered as text.
    #[non_exhaustive]
    I16(i16),
    /// An `i32` rendered as text.
    #[non_exhaustive]
    I32(i32),
    /// An `i64` rendered as text.
    #[non_exhaustive]
    I64(i64),
    /// An `isize` rendered as text.
    #[non_exhaustive]
    Isize(isize),
    /// A `u8` rendered as text.
    #[non_exhaustive]
    U8(u8),
    /// A `u16` rendered as text.
    #[non_exhaustive]
    U16(u16),
    /// A `u32` rendered as text.
    #[non_exhaustive]
    U32(u32),
    /// A `u64` rendered as text.
    #[non_exhaustive]
    U64(u64),
    /// A `usize` rendered as text.
    #[non_exhaustive]
    Usize(usize),
    /// An `f32` rendered as text.
    #[non_exhaustive]
    F32(f32),
    /// An `f64` rendered as text.
    #[non_exhaustive]
    F64(f64),
    /// A character rendered for the recorded context.
    Char { value: char, context: HtmlContext },

    /// A static string and its context.
    StaticStr { ptr: usize, context: HtmlContext },
    /// A dynamic string and its context.
    String { ptr: usize, context: HtmlContext },

    /// A response status code recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    StatusCode(http::StatusCode),
    /// Response headers recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    Headers { ptr: usize },
}

impl Instruction {
    pub fn execute(&self, rom: &ReadOnlyMemory, f: &mut Formatter<'_>) {
        use std::fmt::Write;
        let mut int_buffer = itoa::Buffer::new();

        match self {
            Self::Bool(inner) => f.write_str(if *inner { "true" } else { "false" }),
            // The `Display` output of the numeric types consists of digits,
            // signs, and plain letters, none of which are significant in any
            // HTML context, so they write verbatim.
            Self::I8(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I16(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I32(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I64(inner) => f.write_str(int_buffer.format(*inner)),
            Self::Isize(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U8(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U16(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U32(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U64(inner) => f.write_str(int_buffer.format(*inner)),
            Self::Usize(inner) => f.write_str(int_buffer.format(*inner)),
            Self::F32(inner) => write!(f, "{inner}").unwrap(),
            Self::F64(inner) => write!(f, "{inner}").unwrap(),
            Self::Char { value, context } => context.writer(f).write_char(*value),
            Self::StaticStr { ptr, context } => {}
            Self::Str { value, context } => context.writer(f).write_str(&value),
            Self::BoxDyn { inner, context, .. } => inner.render(cx, &mut context.writer(f)),
            Self::BoxSlice { inner, .. } => {
                for part in inner {
                    part.render(cx, f);
                }
            }
            #[cfg(feature = "http")]
            Self::StatusCode(status_code) => f.record_status_code(status_code),
            #[cfg(feature = "http")]
            Self::Headers(headers) => f.record_headers(*headers),
        }
    }
}

const _: () = {
    assert!(
        !std::mem::needs_drop::<Instruction>(),
        "instruction should not require Drop to improve performance"
    );
    assert!(
        std::mem::size_of::<Instruction>() <= 16,
        "instruction should not exceed 16 bytes"
    );
};
