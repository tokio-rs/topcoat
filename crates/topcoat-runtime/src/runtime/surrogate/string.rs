use ref_cast::RefCast;
use topcoat_view::runtime::{Unescaped, ViewParts};

use crate::runtime::{JsViewParts, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
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

impl JsViewParts for String {
    fn to_view_parts(&self, parts: &mut ViewParts) {
        let inner = &self.0;
        let escaped = format!("{inner:?}");
        parts.push(Unescaped::new_unchecked("cx.s.String("));
        parts.push(escaped);
        parts.push(Unescaped::new_unchecked(")"));
    }
}

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
