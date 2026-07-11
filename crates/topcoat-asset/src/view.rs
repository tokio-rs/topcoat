use topcoat_core::runtime::context::{Cx, RenderContext};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, FmtHtml, Formatter, ViewParts};

use crate::{Asset, AssetRouteResolver, bundled_asset};

impl FmtHtml for Asset {
    fn fmt_html(&self, cx: &RenderContext, f: &mut Formatter<'_>) {
        let bundled_asset = bundled_asset(cx, *self);
        let _ = cx
            .app_context()
            .require::<AssetRouteResolver>()
            .resolve(bundled_asset, f);
    }
}

impl AttributeValueViewParts for Asset {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut ViewParts) {
        parts.push(Box::new(self) as Box<dyn DynViewPart>);
    }
}
