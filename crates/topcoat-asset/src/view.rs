use topcoat_core::context::{Cx, try_app_context};
use topcoat_view::{AttributeValueViewParts, DynViewPart, HtmlWriter, PartsWriter};

use crate::{Asset, AssetConfig};

impl DynViewPart for Asset {
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>) {
        let asset_config = try_app_context::<AssetConfig>(cx).unwrap_or_else(|| {
            panic!("no asset config registered in this router context; load the asset bundle with `.assets(AssetBundle::load().unwrap())`");
        });
        let _ = asset_config.fmt_url(*self, w);
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
