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
use quote::{ToTokens, quote};
use syn::Ident;
use syn::parse::{Parse, ParseStream};

use topcoat_core::ast::ParseOption;

use crate::ast::view::{ViewWriter, WriteView, parse_leading_cx};

/// The full list of attributes attached to a single tag.
pub struct Attributes {
    /// The request context binding supplied by a leading `cx,` argument to the
    /// `attributes!` macro.
    ///
    /// Inside a `#[component]`, `#[page]`, or `#[layout]`, the context is
    /// available implicitly, so this is [`None`]. Anywhere else the caller names
    /// it explicitly as `attributes! { cx, ... }`, mirroring `view! { cx, ... }`.
    /// Attributes parsed as part of an element tag never carry one.
    pub cx: Option<Ident>,
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
        let attrs = writer.into_token_stream();

        // When an explicit context is named, bind it to the `__cx` identifier
        // the generated `__attrs.insert(__cx, ...)` calls read from. Inside a
        // component/page/layout this binding is already in scope, so we emit the
        // attributes untouched.
        match &self.cx {
            Some(cx) => quote! {{
                let __cx: &::topcoat::context::Cx = #cx;
                #attrs
            }}
            .to_tokens(tokens),
            None => attrs.to_tokens(tokens),
        }
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cx = parse_leading_cx(input);
        let mut items = Vec::new();
        while let Some(item) = input.call(AttributeNode::parse_option)? {
            items.push(item);
        }
        Ok(Self { cx, items })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attributes {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        if let Some(cx) = &self.cx {
            cx.pretty_print(printer);
            ",".pretty_print(printer);
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
        }
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
