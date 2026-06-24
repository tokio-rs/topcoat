mod attribute;
mod component;
mod element;
mod format;
mod node;
mod props;
mod unescaped;
mod view;

pub use attribute::*;
pub use component::*;
pub use element::*;
pub use format::*;
pub use node::*;
pub use props::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal {
    use crate::runtime::{
        Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
        ElementNameViewParts, NodeViewParts, Unescaped, ViewParts,
    };

    #[inline]
    pub fn __unescaped(parts: &mut ViewParts, s: &'static str) {
        NodeViewParts::into_view_parts(Unescaped::new_unchecked(s), parts);
    }

    #[inline]
    pub fn __attribute(
        parts: &mut ViewParts,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        Attribute::new(key, value).into_view_parts(parts);
    }

    #[inline]
    pub fn __attribute_unescaped(
        parts: &mut ViewParts,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        Attribute::new(Unescaped::new_unchecked(key), value).into_view_parts(parts);
    }

    #[inline]
    pub fn __attribute_key(parts: &mut ViewParts, attribute_key: impl AttributeKeyViewParts) {
        attribute_key.into_view_parts(parts);
    }

    #[inline]
    pub fn __attribute_value(parts: &mut ViewParts, attribute_value: impl AttributeValueViewParts) {
        attribute_value.into_view_parts(parts);
    }

    #[inline]
    pub fn __attributes(parts: &mut ViewParts, attributes: impl AttributeViewParts) {
        attributes.into_view_parts(parts);
    }

    #[inline]
    pub fn __element_name(parts: &mut ViewParts, element_name: impl ElementNameViewParts) {
        element_name.into_view_parts(parts);
    }

    #[inline]
    pub fn __node(parts: &mut ViewParts, node: impl NodeViewParts) {
        node.into_view_parts(parts);
    }
}
