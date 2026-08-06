pub struct StaticStrPtr(usize);

pub struct StringPtr(usize);

#[cfg(feature = "http")]
pub struct HeadersPtr(usize);

pub struct ReadOnlyMemory {
    static_strs: Vec<&'static str>,
    strings: Vec<String>,
    #[cfg(feature = "http")]
    headers: Vec<http::HeaderMap>,
}

impl ReadOnlyMemory {
    pub fn push_static_str(&mut self, value: &'static str) -> StaticStrPtr {
        self.static_strs.push(value);
        StaticStrPtr(self.static_strs.len() - 1)
    }

    pub fn fetch_static_str(&self, ptr: StaticStrPtr) -> &'static str {
        self.static_strs[ptr.0]
    }

    pub fn push_string(&mut self, value: String) -> StringPtr {
        self.strings.push(value);
        StringPtr(self.strings.len() - 1)
    }

    pub fn fetch_string(&self, ptr: StringPtr) -> &str {
        &self.strings[ptr.0]
    }

    #[cfg(feature = "http")]
    pub fn push_headers(&mut self, value: http::HeaderMap) -> HeadersPtr {
        self.headers.push(value);
        HeadersPtr(self.headers.len() - 1)
    }

    #[cfg(feature = "http")]
    pub fn fetch_headers(&self, ptr: HeadersPtr) -> &http::HeaderMap {
        &self.headers[ptr.0]
    }
}
