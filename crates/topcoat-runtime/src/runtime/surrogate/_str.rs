use ref_cast::RefCast;
use topcoat_view::runtime::{Unescaped, ViewParts};

use crate::runtime::{JsViewParts, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Str(str);

impl_surrogate_ref!(str, Str);
impl_surrogate_mut!(str, Str);

impl JsViewParts for Str {
    fn to_view_parts(&self, parts: &mut ViewParts) {
        let inner = &self.0;
        let escaped = format!("{inner:?}");
        parts.push(Unescaped::new_unchecked("cx.s.str("));
        parts.push(escaped);
        parts.push(Unescaped::new_unchecked(")"));
    }
}

impl std::fmt::Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
