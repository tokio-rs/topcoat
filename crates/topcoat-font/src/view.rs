use topcoat_core::context::{Cx, context};
use topcoat_view::{AttributeValueViewParts, PartsWriter};

use crate::{Font, FontResolver};

impl AttributeValueViewParts for Font {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let mut url = String::new();
        let _ = context::<FontResolver>(cx).resolve(self, &mut url);
        parts.push_string(url);
    }
}
