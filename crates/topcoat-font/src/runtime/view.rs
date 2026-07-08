use topcoat_core::runtime::context::{Cx, app_context};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, FmtHtml, Formatter, ViewParts};

use crate::runtime::{Font, FontResolver};

impl FmtHtml for Font {
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        let _ = app_context::<FontResolver>(cx).resolve(*self, f);
    }
}

impl AttributeValueViewParts for Font {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut ViewParts) {
        parts.push(Box::new(self) as Box<dyn DynViewPart>);
    }
}
