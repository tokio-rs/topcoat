use topcoat_core::runtime::context::{Cx, app_context};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, HtmlWriter, PartsWriter};

use crate::runtime::{Font, FontResolver};

impl DynViewPart for Font {
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>) {
        let _ = app_context::<FontResolver>(cx).resolve(*self, w);
    }

    #[inline]
    fn clone_box(&self) -> Box<dyn DynViewPart> {
        Box::new(*self)
    }
}

impl AttributeValueViewParts for Font {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_dyn(Box::new(self));
    }
}
