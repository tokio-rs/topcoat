#[cfg(feature = "http")]
use crate::render::HeadersPtr;
use crate::{
    HtmlContext,
    render::{DynPtr, InstructionPtr, StaticStrPtr, StringPtr},
};

#[derive(Debug, Clone)]
pub enum Instruction {
    /// Jump into a nested block, returning here at its [`Ret`](Self::Ret).
    Call { entry: InstructionPtr },
    /// Return back to the previous call instruction, if any.
    Ret,
    /// Jump to `entry` without recording a return address.
    Jmp { entry: InstructionPtr },
    /// Holds a reserved slot until it is filled; executing it panics.
    Placeholder,

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
    StaticStr {
        ptr: StaticStrPtr,
        context: HtmlContext,
    },
    /// A string copied into the pool's string arena, and its context.
    ///
    /// The arena location is inlined instead of held as a
    /// [`StrPtr`](crate::render::StrPtr), whose trailing padding would push
    /// the variant past the instruction size limit.
    #[non_exhaustive]
    Str {
        offset: usize,
        len: u32,
        context: HtmlContext,
    },
    /// A dynamic string and its context.
    String {
        ptr: StringPtr,
        context: HtmlContext,
    },
    /// A part that writes its output at render time, and its context.
    Dyn { ptr: DynPtr, context: HtmlContext },

    /// A response status code recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    StatusCode(http::StatusCode),
    /// Response headers recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    Headers { ptr: HeadersPtr },
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
