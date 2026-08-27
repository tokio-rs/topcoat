use std::sync::Arc;

use crate::{
    DynViewPart,
    buffer::{InstructionPtr, ViewBuffer},
};

/// The index of a static string in a [`ConstBuffer`].
#[derive(Debug, Clone, Copy)]
pub(super) struct StaticStrPtr(usize);

/// The index of an owned string in a [`ConstBuffer`].
#[derive(Debug, Clone, Copy)]
pub(super) struct StringPtr(usize);

/// The location of a string copied into a [`ConstBuffer`]'s string storage.
#[derive(Debug, Clone, Copy)]
pub(super) struct StrPtr {
    pub(super) offset: usize,
    pub(super) len: u32,
}

/// The index of a boxed [`DynViewPart`] in a [`ConstBuffer`].
#[derive(Debug, Clone, Copy)]
pub(super) struct DynPtr(usize);

/// The index of an owned view's buffer and entry in a [`ConstBuffer`].
#[derive(Debug, Clone, Copy)]
pub(super) struct ViewPtr(usize);

/// The index of a header map in a [`ConstBuffer`].
#[cfg(feature = "http")]
#[derive(Debug, Clone, Copy)]
pub(super) struct HeadersPtr(usize);

/// The constants of a [`ViewBuffer`](crate::buffer::ViewBuffer): the
/// out-of-line operands the instruction sequence refers to by index.
///
/// Instructions are fixed-size, so any operand that does not fit inline, such
/// as a string or a header map, is pushed in here and referenced through a
/// typed pointer.
#[derive(Debug, Default)]
pub(super) struct ConstBuffer {
    static_strs: Vec<&'static str>,
    strings: Vec<String>,
    strs: String,
    dyns: Vec<Box<dyn DynViewPart>>,
    views: Vec<(Arc<ViewBuffer>, InstructionPtr)>,
    #[cfg(feature = "http")]
    headers: Vec<http::HeaderMap>,
}

impl ConstBuffer {
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(super) fn push_static_str(&mut self, value: &'static str) -> StaticStrPtr {
        self.static_strs.push(value);
        StaticStrPtr(self.static_strs.len() - 1)
    }

    #[inline]
    #[must_use]
    pub(super) fn fetch_static_str(&self, ptr: StaticStrPtr) -> &'static str {
        self.static_strs[ptr.0]
    }

    #[inline]
    pub(super) fn push_string(&mut self, value: String) -> StringPtr {
        self.strings.push(value);
        StringPtr(self.strings.len() - 1)
    }

    #[inline]
    #[must_use]
    pub(super) fn fetch_string(&self, ptr: StringPtr) -> &str {
        &self.strings[ptr.0]
    }

    /// Copies a borrowed string to the end of the string storage.
    ///
    /// # Panics
    ///
    /// Panics if the string is longer than `u32::MAX` bytes.
    #[inline]
    pub(super) fn push_str(&mut self, value: &str) -> StrPtr {
        let offset = self.strs.len();
        let len = u32::try_from(value.len()).expect("string exceeds the buffer's length limit");
        self.strs.push_str(value);
        StrPtr { offset, len }
    }

    #[inline]
    #[must_use]
    pub(super) fn fetch_str(&self, ptr: StrPtr) -> &str {
        &self.strs[ptr.offset..ptr.offset + ptr.len as usize]
    }

    #[inline]
    pub(super) fn push_dyn(&mut self, value: Box<dyn DynViewPart>) -> DynPtr {
        self.dyns.push(value);
        DynPtr(self.dyns.len() - 1)
    }

    #[inline]
    #[must_use]
    pub(super) fn fetch_dyn(&self, ptr: DynPtr) -> &dyn DynViewPart {
        &*self.dyns[ptr.0]
    }

    /// Stores the buffer an owned view holds together with the entry of its
    /// instruction block.
    #[inline]
    pub(super) fn push_view(&mut self, buffer: Arc<ViewBuffer>, entry: InstructionPtr) -> ViewPtr {
        self.views.push((buffer, entry));
        ViewPtr(self.views.len() - 1)
    }

    #[inline]
    #[must_use]
    pub(super) fn fetch_view(&self, ptr: ViewPtr) -> (&Arc<ViewBuffer>, InstructionPtr) {
        let (buffer, entry) = &self.views[ptr.0];
        (buffer, *entry)
    }

    #[cfg(feature = "http")]
    #[inline]
    pub(super) fn push_headers(&mut self, value: http::HeaderMap) -> HeadersPtr {
        self.headers.push(value);
        HeadersPtr(self.headers.len() - 1)
    }

    #[cfg(feature = "http")]
    #[inline]
    #[must_use]
    pub(super) fn fetch_headers(&self, ptr: HeadersPtr) -> &http::HeaderMap {
        &self.headers[ptr.0]
    }

    /// Prints how many operands of each kind the buffer holds.
    #[allow(unused)]
    pub(super) fn print_stats(&self) {
        println!("ConstBuffer {{");
        println!("  static_strs: {}", self.static_strs.len());
        println!("  strings: {}", self.strings.len());
        println!("  strs: {} bytes", self.strs.len());
        println!("  dyns: {}", self.dyns.len());
        #[cfg(feature = "http")]
        println!("  headers: {}", self.headers.len());
        println!("}}");
    }
}
