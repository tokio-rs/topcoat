//! Runtime types used by generated views and components.
//!
//! Most applications use this module through Topcoat's view and component
//! macros. The public traits are available for custom values that need to be
//! accepted in node, element-name, or attribute positions.

mod attribute;
mod component;
mod element;
mod format;
mod node;
mod unescaped;
mod view;

pub use attribute::*;
pub use component::*;
pub use element::*;
pub use format::*;
pub use node::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal {
    use crate::runtime::{
        Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
        ElementNameViewParts, NodeViewParts, Unescaped, ViewParts,
    };

    #[inline(always)]
    pub fn __unescaped(parts: &mut ViewParts, s: &'static str) {
        NodeViewParts::into_view_parts(Unescaped::new_unchecked(s), parts);
    }

    #[inline(always)]
    pub fn __attribute(
        parts: &mut ViewParts,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        Attribute::new(key, value).into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __attribute_unescaped(
        parts: &mut ViewParts,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        Attribute::new(Unescaped::new_unchecked(key), value).into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __attribute_key(parts: &mut ViewParts, attribute_key: impl AttributeKeyViewParts) {
        attribute_key.into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __attribute_value(parts: &mut ViewParts, attribute_value: impl AttributeValueViewParts) {
        attribute_value.into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __attributes(parts: &mut ViewParts, attributes: impl AttributeViewParts) {
        attributes.into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __element_name(parts: &mut ViewParts, element_name: impl ElementNameViewParts) {
        element_name.into_view_parts(parts);
    }

    #[inline(always)]
    pub fn __node(parts: &mut ViewParts, node: impl NodeViewParts) {
        node.into_view_parts(parts);
    }
}
