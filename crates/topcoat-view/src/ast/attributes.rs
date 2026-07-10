mod attribute;
mod attribute_key;
mod attribute_node;
mod attribute_nodes;
mod attribute_spread;
mod attribute_value;
mod attribute_writer;
mod bind_attribute;
mod event_handler;
mod visitor;

use std::collections::HashSet;

pub use attribute::*;
pub use attribute_key::*;
pub use attribute_node::*;
pub use attribute_nodes::*;
pub use attribute_spread::*;
pub use attribute_value::*;
pub(crate) use attribute_writer::*;
pub use bind_attribute::*;
pub use event_handler::*;
pub use visitor::*;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::Ident;
use syn::parse::{Parse, ParseStream};

use topcoat_core::ast::ParseOption;

use crate::ast::template::{TemplateElse, TemplateForLoop, TemplateIf, TemplateMatch};
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

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cx = parse_leading_cx(input);
        let mut items = Vec::new();
        while let Some(item) = input.call(AttributeNode::parse_option)? {
            items.push(item);
        }

        // Check for uniqueness of attribute keys.
        let mut visitor = DuplicateAttributesVisitor::default();
        for item in &items {
            visitor.visit_node(item);
        }
        if let Some(error) = visitor.error {
            return Err(error);
        }

        Ok(Self { cx, items })
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

#[derive(Default)]
struct DuplicateAttributesVisitor {
    attributes: HashSet<String>,
    error: Option<syn::Error>,
}

impl DuplicateAttributesVisitor {
    fn register(&mut self, key: String, span: Span) {
        if self.error.is_some() {
            return;
        }
        if self.attributes.contains(&key) {
            self.error = Some(syn::Error::new(
                span,
                format!("duplicate attribute `{key}`"),
            ));
        }
        self.attributes.insert(key);
    }
}

impl<'ast> Visit<'ast> for DuplicateAttributesVisitor {
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        if let Some(ident) = node.key.as_ident() {
            self.register(ident.to_string(), ident.span());
        }
    }

    fn visit_bind_attribute(&mut self, node: &'ast BindAttribute) {
        if let Some(ident) = node.key.as_ident() {
            self.register(format!(":{ident}"), ident.span());
        }
    }

    fn visit_event_handler(&mut self, node: &'ast EventHandler) {
        if let Some(ident) = node.key.as_ident() {
            self.register(format!("@{ident}"), ident.span());
        }
    }

    fn visit_for_loop(&mut self, node: &'ast TemplateForLoop<AttributeNodes>) {
        if self.error.is_some() {
            return;
        }

        let mut visitor = DuplicateAttributesVisitor::default();
        visit_for_loop(&mut visitor, node);
        self.error = visitor.error;
    }

    fn visit_if(&mut self, node: &'ast TemplateIf<AttributeNodes>) {
        if self.error.is_some() {
            return;
        }

        let mut visitor = DuplicateAttributesVisitor::default();
        for node in &node.then_branch.children {
            visitor.visit_node(node);
        }
        self.error = visitor.error;

        if let Some(else_branch) = &node.else_branch {
            self.visit_else(else_branch);
        }
    }

    fn visit_else(&mut self, node: &'ast TemplateElse<AttributeNodes>) {
        if self.error.is_some() {
            return;
        }

        match node {
            TemplateElse::ElseIf { template_if, .. } => self.visit_if(template_if),
            TemplateElse::Else { then_branch, .. } => {
                let mut visitor = DuplicateAttributesVisitor::default();
                for node in &then_branch.children {
                    visitor.visit_node(node);
                }
                self.error = visitor.error;
            }
        }
    }

    fn visit_match(&mut self, node: &'ast TemplateMatch<AttributeNode>) {
        for arm in &node.arms {
            if self.error.is_some() {
                return;
            }

            let mut visitor = DuplicateAttributesVisitor::default();
            visitor.visit_node(&arm.body);
            self.error = visitor.error;
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

    #[test]
    #[should_panic(expected = "duplicate attribute `dup-attr`")]
    fn errors_on_duplicate_attribute() {
        let _ = parse(r#"not-dup="5" dup-attr="6" dup-attr="7" another="8""#);
    }

    #[test]
    #[should_panic(expected = "duplicate attribute `:dup-attr`")]
    fn errors_on_duplicate_bind_attribute() {
        let _ = parse(r":not-dup=(()) :dup-attr=(()) :dup-attr=(()) :another=(())");
    }

    #[test]
    #[should_panic(expected = "duplicate attribute `@dup-attr`")]
    fn errors_on_duplicate_event_handler() {
        let _ = parse(r"@not-dup=(()) @dup-attr=(()) @dup-attr=(()) @another=(())");
    }
}
