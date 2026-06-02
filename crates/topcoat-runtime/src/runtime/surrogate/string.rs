use std::fmt::Write;

use ref_cast::RefCast;

use crate::runtime::{ToJs, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct String(std::string::String);

impl String {
    #[inline]
    pub(crate) const fn new(v: std::string::String) -> Self {
        Self(v)
    }
}

impl_surrogate!(std::string::String, String);
impl_surrogate_ref!(std::string::String, String);
impl_surrogate_mut!(std::string::String, String);

impl ToJs for String {
    fn to_js(&self, out: &mut std::string::String) {
        let inner = self.0.as_str();
        let _ = write!(out, "cx.s.String({inner:?})");
    }
}

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
