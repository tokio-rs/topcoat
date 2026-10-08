use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Direction, LanguageIdentifier, Locale};

impl AttributeValueViewParts for Direction {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str(self.as_promoted_str());
    }
}

impl AttributeValueViewParts for &LanguageIdentifier {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.to_string());
    }
}

impl AttributeValueViewParts for LanguageIdentifier {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        AttributeValueViewParts::into_view_parts(&self, cx, parts);
    }
}

/// Implements the node view trait for a type and references to it by
/// rendering its [`Display`](std::fmt::Display) text.
macro_rules! impl_node_view_parts {
    ($ty:ty) => {
        impl NodeViewParts for &$ty {
            #[inline]
            fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
                parts.push_string(self.to_string());
            }
        }

        impl NodeViewParts for $ty {
            #[inline]
            fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
                NodeViewParts::into_view_parts(&self, cx, parts);
            }
        }
    };
}

impl_node_view_parts!(LanguageIdentifier);
impl_node_view_parts!(Locale);
