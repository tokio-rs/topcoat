#[cfg(feature = "http")]
use crate::buffer::HeadersPtr;
use crate::{
    HtmlContext,
    buffer::{DynPtr, InstructionPtr, StaticStrPtr, StringPtr, ViewPtr},
};

#[derive(Debug, Clone)]
pub(super) enum Instruction {
    /// Jump into a nested block, returning here at its [`Ret`](Self::Ret).
    Call { entry: InstructionPtr },
    /// Return back to the previous call instruction, if any.
    Ret,
    /// Continue at `entry` without recording a return address: the rest of
    /// a block that was suspended and resumed elsewhere in the buffer.
    Jmp { entry: InstructionPtr },
    /// A node position whose view has not resolved yet; executing it
    /// panics.
    Placeholder,
    /// Execute a spliced owned view's block in the buffer it carries.
    ViewHandle { ptr: ViewPtr },

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

    /// A static string held by reference, and its context.
    ///
    /// The string is held as a reference to a `&'static str` so the operand
    /// stays one word wide, since the pointer and length pair it refers to
    /// lives in the binary's read-only data. Callers pass `&"..."`, which
    /// Rust promotes to the required reference.
    PromotedStr {
        value: &'static &'static str,
        context: HtmlContext,
    },
    /// A static string recorded in the buffer's constants, and its context.
    ///
    /// A `&'static str` that is only known at run time cannot be promoted, so
    /// it is held out of line instead. A string written as a literal should go
    /// through [`PromotedStr`](Self::PromotedStr).
    StaticStr {
        ptr: StaticStrPtr,
        context: HtmlContext,
    },
    /// A string copied into the buffer's string storage, and its context.
    ///
    /// The location is inlined instead of held as a
    /// [`StrPtr`](crate::buffer::StrPtr), whose trailing padding would push
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
