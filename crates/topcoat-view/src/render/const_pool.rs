use crate::DynViewPart;

/// The index of a static string in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct StaticStrPtr(usize);

/// The index of an owned string in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct StringPtr(usize);

/// The index of a boxed [`DynViewPart`] in a [`ConstPool`].
#[derive(Debug, Clone, Copy)]
pub struct DynPtr(usize);

/// The index of a header map in a [`ConstPool`].
#[cfg(feature = "http")]
#[derive(Debug, Clone, Copy)]
pub struct HeadersPtr(usize);

/// The constant pool of a [`Memory`](crate::render::Memory): the out-of-line
/// operands the instruction sequence refers to by index.
///
/// Instructions are fixed-size, so any operand that does not fit inline, such
/// as a string or a header map, is pushed into the pool and referenced
/// through a typed pointer.
#[derive(Debug, Default)]
pub struct ConstPool {
    static_strs: Vec<&'static str>,
    strings: Vec<String>,
    dyns: Vec<Box<dyn DynViewPart>>,
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

    pub fn push_dyn(&mut self, value: Box<dyn DynViewPart>) -> DynPtr {
        self.dyns.push(value);
        DynPtr(self.dyns.len() - 1)
    }

    #[must_use]
    pub fn fetch_dyn(&self, ptr: DynPtr) -> &dyn DynViewPart {
        &*self.dyns[ptr.0]
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
}
