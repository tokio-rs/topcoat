use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, PartsWriter};

use crate::{Asset, asset_config};

impl AttributeValueViewParts for Asset {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    /// Resolves the asset's URL with the context the view was built with, so
    /// a [`UrlForm`](topcoat_core::url_form::UrlForm) set for the view
    /// applies.
    #[track_caller]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(asset_config(cx).resolve(cx, self));
    }
}
