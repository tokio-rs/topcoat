mod attribute;
mod attribute_key;
mod attribute_node;
mod attribute_nodes;
mod attribute_spread;
mod attribute_value;
mod attribute_writer;
mod bind_attribute;
mod event_handler;

pub use attribute::*;
pub use attribute_key::*;
pub use attribute_node::*;
pub use attribute_nodes::*;
pub use attribute_spread::*;
pub use attribute_value::*;
pub(crate) use attribute_writer::*;
pub use bind_attribute::*;
pub use event_handler::*;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use topcoat_core::ast::ParseOption;

use crate::ast::view::{ViewWriter, WriteView};

/// The full list of attributes attached to a single tag.
pub struct Attributes {
    pub items: Vec<AttributeNode>,
}

impl Attributes {
    /// Returns `true` if `self` has no attributes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl WriteView for Attributes {
    fn write(&self, writer: &mut ViewWriter) {
        for item in &self.items {
            WriteView::write(item, writer);
        }
    }
}

impl WriteAttribute for Attributes {
    fn write(&self, writer: &mut AttributeWriter) {
        for item in &self.items {
            WriteAttribute::write(item, writer);
        }
    }
}

impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut writer = AttributeWriter::new();
        WriteAttribute::write(self, &mut writer);
        writer.into_token_stream().to_tokens(tokens);
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while let Some(item) = input.call(AttributeNode::parse_option)? {
            items.push(item);
        }
        Ok(Self { items })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attributes {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        if self.is_empty() {
            return;
        }
        for (index, item) in self.items.iter().enumerate() {
            item.pretty_print(printer);
            if index < self.items.len() - 1 {
                printer.scan_break();
                " ".pretty_print(printer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Attributes {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn empty_input_yields_no_items() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn collects_sibling_nodes_in_order() {
        let attrs = parse(r#"class="button" id=(id) :value=$(value) @input="handle()""#);
        assert_eq!(attrs.items.len(), 4);
        assert!(matches!(attrs.items[0], AttributeNode::Attribute(_)));
        assert!(matches!(attrs.items[1], AttributeNode::Attribute(_)));
        assert!(matches!(attrs.items[2], AttributeNode::BindAttribute(_)));
        assert!(matches!(attrs.items[3], AttributeNode::EventHandler(_)));
    }

    #[test]
    fn collects_control_flow_nodes() {
        let attrs = parse(
            r#"
                let active = true;
                if active { class="active" } else { class="inactive" }
                for (key, value) in attrs { (key)=(value) }
                match kind {
                    "button" => role="button",
                    _ => data-kind=(kind),
                }
            "#,
        );
        assert_eq!(attrs.items.len(), 4);
        assert!(matches!(attrs.items[0], AttributeNode::Let(_)));
        assert!(matches!(attrs.items[1], AttributeNode::If(_)));
        assert!(matches!(attrs.items[2], AttributeNode::ForLoop(_)));
        assert!(matches!(attrs.items[3], AttributeNode::Match(_)));
    }
}
