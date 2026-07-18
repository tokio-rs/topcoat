#![cfg_attr(docsrs, feature(doc_cfg))]

mod attribute;
mod class;
mod component;
mod element;
mod escape;
mod format;
mod length;
mod node;
mod props;
pub mod svg;
mod unescaped;
mod view;

pub use attribute::*;
pub use class::*;
pub use component::*;
pub use element::*;
pub use escape::*;
pub use format::*;
pub use length::*;
pub use node::*;
pub use props::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal {
    use topcoat_core::context::Cx;

    use crate::{
        Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
        ElementNameViewParts, HtmlContext, NodeViewParts, PartsWriter, Unescaped, View, ViewParts,
    };

    #[inline]
    pub fn __unescaped(_cx: &Cx, parts: &mut ViewParts, s: &'static str) {
        PartsWriter::new(parts, HtmlContext::Unescaped).push_str(s);
    }

    #[inline]
    pub fn __view(_cx: &Cx, parts: &mut ViewParts, view: View) {
        parts.push_view(view);
    }

    #[inline]
    pub fn __node(cx: &Cx, parts: &mut ViewParts, node: impl NodeViewParts) {
        node.into_view_parts(cx, &mut PartsWriter::new(parts, HtmlContext::Text));
    }

    #[inline]
    pub fn __element_name(cx: &Cx, parts: &mut ViewParts, element_name: impl ElementNameViewParts) {
        element_name.into_view_parts(cx, &mut PartsWriter::new(parts, HtmlContext::ElementName));
    }

    #[inline]
    pub fn __attribute_key(
        cx: &Cx,
        parts: &mut ViewParts,
        attribute_key: impl AttributeKeyViewParts,
    ) {
        attribute_key.into_view_parts(cx, &mut PartsWriter::new(parts, HtmlContext::AttributeKey));
    }

    #[inline]
    pub fn __attribute_value(
        cx: &Cx,
        parts: &mut ViewParts,
        attribute_value: impl AttributeValueViewParts,
    ) {
        attribute_value.into_view_parts(
            cx,
            &mut PartsWriter::new(parts, HtmlContext::AttributeValue),
        );
    }

    #[inline]
    pub fn __attribute(
        cx: &Cx,
        parts: &mut ViewParts,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        __attributes(cx, parts, Attribute::new(key, value));
    }

    #[inline]
    pub fn __attribute_unescaped(
        cx: &Cx,
        parts: &mut ViewParts,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        __attributes(
            cx,
            parts,
            Attribute::new(Unescaped::new_unchecked(key), value),
        );
    }

    #[inline]
    pub fn __attributes(cx: &Cx, parts: &mut ViewParts, attributes: impl AttributeViewParts) {
        // Whole-attribute values do their own context transitions between
        // keys and values; the attribute-value context here is the safe
        // default for any text pushed directly.
        attributes.into_view_parts(
            cx,
            &mut PartsWriter::new(parts, HtmlContext::AttributeValue),
        );
    }
}
