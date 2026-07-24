use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, DynViewPart, HtmlWriter, PartsWriter};

use crate::{Asset, asset_config};

impl DynViewPart for Asset {
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>) {
        let _ = asset_config(cx).fmt_url(*self, w);
    }

    #[inline]
    fn clone_box(&self) -> Box<dyn DynViewPart> {
        Box::new(*self)
    }
}

impl AttributeValueViewParts for Asset {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_dyn(Box::new(self));
    }
}
