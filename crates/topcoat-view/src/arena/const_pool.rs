use std::sync::Arc;

use crate::{
    DynViewPart,
    arena::{Arena, InstructionPtr},
};

/// The index of a static string in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct StaticStrPtr(usize);

/// The index of an owned string in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct StringPtr(usize);

/// The location of a string copied into a [`ConstPool`]'s string arena.
#[derive(Debug, Clone, Copy)]
pub struct StrPtr {
    pub(crate) offset: usize,
    pub(crate) len: u32,
}

/// The index of a boxed [`DynViewPart`] in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct DynPtr(usize);

/// The index of an owned view's arena and entry in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct ViewPtr(usize);

/// The index of a header map in a [`ConstPool`].
#[cfg(feature = "http")]
#[derive(Debug, Clone, Copy)]
pub struct HeadersPtr(usize);

/// The constant pool of an [`Arena`](crate::arena::Arena): the out-of-line
/// operands the instruction sequence refers to by index.
///
/// Instructions are fixed-size, so any operand that does not fit inline, such
/// as a string or a header map, is pushed into the pool and referenced
/// through a typed pointer.
#[derive(Debug, Default)]
pub struct ConstPool {
    static_strs: Vec<&'static str>,
    strings: Vec<String>,
    strs: String,
    dyns: Vec<Box<dyn DynViewPart>>,
    views: Vec<(Arc<Arena>, InstructionPtr)>,
    #[cfg(feature = "http")]
    headers: Vec<http::HeaderMap>,
}

impl ConstPool {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_static_str(&mut self, value: &'static str) -> StaticStrPtr {
        self.static_strs.push(value);
        StaticStrPtr(self.static_strs.len() - 1)
    }

    #[must_use]
    pub fn fetch_static_str(&self, ptr: StaticStrPtr) -> &'static str {
        self.static_strs[ptr.0]
    }

    pub fn push_string(&mut self, value: String) -> StringPtr {
        self.strings.push(value);
        StringPtr(self.strings.len() - 1)
    }

    #[must_use]
    pub fn fetch_string(&self, ptr: StringPtr) -> &str {
        &self.strings[ptr.0]
    }

    /// Copies a borrowed string to the end of the string arena.
    ///
    /// # Panics
    ///
    /// Panics if the string is longer than `u32::MAX` bytes.
    pub fn push_str(&mut self, value: &str) -> StrPtr {
        let offset = self.strs.len();
        let len = u32::try_from(value.len()).expect("string exceeds the arena's length limit");
        self.strs.push_str(value);
        StrPtr { offset, len }
    }

    #[must_use]
    pub fn fetch_str(&self, ptr: StrPtr) -> &str {
        &self.strs[ptr.offset..ptr.offset + ptr.len as usize]
    }

    pub fn push_dyn(&mut self, value: Box<dyn DynViewPart>) -> DynPtr {
        self.dyns.push(value);
        DynPtr(self.dyns.len() - 1)
    }

    #[must_use]
    pub fn fetch_dyn(&self, ptr: DynPtr) -> &dyn DynViewPart {
        &*self.dyns[ptr.0]
    }

    /// Stores the arena an owned view holds together with the entry of its
    /// instruction block.
    pub fn push_view(&mut self, arena: Arc<Arena>, entry: InstructionPtr) -> ViewPtr {
        self.views.push((arena, entry));
        ViewPtr(self.views.len() - 1)
    }

    #[must_use]
    pub fn fetch_view(&self, ptr: ViewPtr) -> (&Arena, InstructionPtr) {
        let (arena, entry) = &self.views[ptr.0];
        (arena, *entry)
    }

    #[cfg(feature = "http")]
    pub fn push_headers(&mut self, value: http::HeaderMap) -> HeadersPtr {
        self.headers.push(value);
        HeadersPtr(self.headers.len() - 1)
    }

    #[cfg(feature = "http")]
    #[must_use]
    pub fn fetch_headers(&self, ptr: HeadersPtr) -> &http::HeaderMap {
        &self.headers[ptr.0]
    }

    /// Prints how many operands of each kind the pool holds.
    #[allow(unused)]
    pub(crate) fn print_stats(&self) {
        println!("ConstPool {{");
        println!("  static_strs: {}", self.static_strs.len());
        println!("  strings: {}", self.strings.len());
        println!("  strs: {} bytes", self.strs.len());
        println!("  dyns: {}", self.dyns.len());
        #[cfg(feature = "http")]
        println!("  headers: {}", self.headers.len());
        println!("}}");
    }
}
