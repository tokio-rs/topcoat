mod attribute;
mod component;
mod element;
mod format;
mod length;
mod node;
mod props;
pub mod svg;
mod unescaped;
mod view;

pub use attribute::*;
pub use component::*;
pub use element::*;
pub use format::*;
pub use length::*;
pub use node::*;
pub use props::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal {
    use topcoat_core::runtime::context::Cx;

    use crate::runtime::{
        Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
        ElementNameViewParts, NodeViewParts, Unescaped, ViewParts,
    };

    #[inline]
    pub fn __unescaped(cx: &Cx, parts: &mut ViewParts, s: &'static str) {
        NodeViewParts::into_view_parts(Unescaped::new_unchecked(s), cx, parts);
    }

    #[inline]
    pub fn __attribute(
        cx: &Cx,
        parts: &mut ViewParts,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        Attribute::new(key, value).into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __attribute_unescaped(
        cx: &Cx,
        parts: &mut ViewParts,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        Attribute::new(Unescaped::new_unchecked(key), value).into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __attribute_key(
        cx: &Cx,
        parts: &mut ViewParts,
        attribute_key: impl AttributeKeyViewParts,
    ) {
        attribute_key.into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __attribute_value(
        cx: &Cx,
        parts: &mut ViewParts,
        attribute_value: impl AttributeValueViewParts,
    ) {
        attribute_value.into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __attributes(cx: &Cx, parts: &mut ViewParts, attributes: impl AttributeViewParts) {
        attributes.into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __element_name(cx: &Cx, parts: &mut ViewParts, element_name: impl ElementNameViewParts) {
        element_name.into_view_parts(cx, parts);
    }

    #[inline]
    pub fn __node(cx: &Cx, parts: &mut ViewParts, node: impl NodeViewParts) {
        node.into_view_parts(cx, parts);
    }
}
