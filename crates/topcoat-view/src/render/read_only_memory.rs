pub struct ReadOnlyMemory {
    static_strs: Vec<&'static str>,
    strings: Vec<String>,
    #[cfg(feature = "http")]
    headers: Vec<http::HeaderMap>,
}

impl ReadOnlyMemory {
    fn resolve_static_str(&self, ptr: usize) -> &'static str {
        self.static_strs[ptr]
    }
}
