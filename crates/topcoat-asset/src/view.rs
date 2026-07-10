use topcoat_core::runtime::context::{Cx, app_context};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, HtmlWriter, PartsWriter};

use crate::{Asset, AssetRouteResolver, bundled_asset};

impl DynViewPart for Asset {
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>) {
        let bundled_asset = bundled_asset(cx, *self);
        let _ = app_context::<AssetRouteResolver>(cx).resolve(bundled_asset, w);
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
