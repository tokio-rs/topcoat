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
use syn::parse::{Parse, ParseStream};

use topcoat_core_grammar::ParseOption;

use crate::leading_cx::LeadingCx;
use crate::template::{TemplateElse, TemplateForLoop, TemplateIf, TemplateMatch};
use crate::view::{self, ViewWriter, WriteView};

/// The full list of attributes attached to a single tag.
pub struct Attributes {
    /// The request context binding supplied by a leading `cx =>` argument to
    /// the `attributes!` macro.
    ///
    /// Inside a `#[component]`, `#[page]`, or `#[layout]`, the context is
    /// available implicitly, so this is [`None`]. Anywhere else the caller names
    /// it explicitly as `attributes! { cx => ... }`, mirroring `view! { cx => ... }`.
    /// Attributes parsed as part of an element tag never carry one.
    pub cx: Option<LeadingCx>,
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
        let mut visitor = DynamicAttributesVisitor::default();
        for item in &self.items {
            visitor.visit_node(item);
        }
        // If we cannot statically assert that all attribute keys are unique we must fall back to a
        // slower runtime map of attributes.
        if visitor.dynamic {
            writer.write_expr(view::ExprKind::Attributes, self.to_token_stream());
        } else {
            for item in &self.items {
                WriteView::write(item, writer);
            }
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
        let cx = input.call(LeadingCx::parse_option)?;
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
                #cx
                #attrs
            }}
            .to_tokens(tokens),
            None => attrs.to_tokens(tokens),
        }
    }
}

impl topcoat_pretty::PrettyPrint for Attributes {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        if let Some(cx) = &self.cx {
            cx.pretty_print(printer);
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

/// Visitor that checks whether attribute key uniqueness can be statically verified or if a runtime
/// `Attributes` map has to be created.
#[derive(Default)]
struct DynamicAttributesVisitor {
    dynamic: bool,
}

impl<'ast> Visit<'ast> for DynamicAttributesVisitor {
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        // Dynamic keys cannot be checked at build time.
        if node.key.is_expr() {
            self.dynamic = true;
        }
    }

    fn visit_bind_attribute(&mut self, node: &'ast BindAttribute) {
        // Dynamic keys cannot be checked at build time.
        if node.key.is_expr() {
            self.dynamic = true;
        }
    }

    fn visit_event_handler(&mut self, node: &'ast EventHandler) {
        // Dynamic keys cannot be checked at build time.
        if node.key.is_expr() {
            self.dynamic = true;
        }
    }

    fn visit_if(&mut self, _node: &'ast TemplateIf<AttributeNodes>) {
        // Multiple conditionals could create the same key.
        self.dynamic = true;
    }

    fn visit_for_loop(&mut self, _node: &'ast TemplateForLoop<AttributeNodes>) {
        // Body could be run twice, creating duplicate attribute.
        self.dynamic = true;
    }

    fn visit_match(&mut self, _node: &'ast TemplateMatch<AttributeNode>) {
        // Multiple conditionals could create the same key.
        self.dynamic = true;
    }

    fn visit_spread(&mut self, _node: &'ast AttributeSpread) {
        // Dynamic keys cannot be checked at build time.
        self.dynamic = true;
    }
}

/// Visitor that returns a correctly spanned error in case of a duplicate attribute.
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

    /// Renders the `WriteView` output of an attribute list to a token string.
    fn write_view(source: &str) -> String {
        let mut writer = ViewWriter::new();
        WriteView::write(&parse(source), &mut writer);
        writer.into_token_stream().to_string()
    }

    /// Returns `true` if the attribute list falls back to a runtime `Attributes`
    /// map instead of emitting a static view.
    ///
    /// The fallback is identified by the `Attributes::with_capacity` call the
    /// runtime map builder always emits; the static path never produces it.
    fn is_dynamic(source: &str) -> bool {
        write_view(source).contains("Attributes :: with_capacity")
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

    #[test]
    fn generates_static_view_for_literal_attributes() {
        // Statically known, unique keys are written straight into the markup.
        let out = write_view(r#"class="button" id="main""#);
        assert!(!out.contains("Attributes :: with_capacity"));
        assert!(out.contains(r#" class=\"button\" id=\"main\""#));
    }

    #[test]
    fn generates_static_view_for_expression_values() {
        // Expression *values* keep static keys, so uniqueness is still provable.
        assert!(!is_dynamic(r#"class="button" href=(url)"#));
    }

    #[test]
    fn generates_static_view_for_bind_and_event_with_static_keys() {
        // Bind attributes and event handlers with static keys stay static.
        assert!(!is_dynamic(r#":value=$(value) @input="handle()""#));
    }

    #[test]
    fn generates_static_view_with_let_binding() {
        // A `let` introduces no key, so surrounding static keys remain provable.
        assert!(!is_dynamic(r#"let cls = "x"; class=(cls)"#));
    }

    #[test]
    fn generates_dynamic_attributes_for_expression_key() {
        // A dynamic key cannot be checked for uniqueness at build time.
        assert!(is_dynamic(r"(name)=(value)"));
    }

    #[test]
    fn generates_dynamic_attributes_for_bind_expression_key() {
        assert!(is_dynamic(r":(name)=$(value)"));
    }

    #[test]
    fn generates_dynamic_attributes_for_event_expression_key() {
        assert!(is_dynamic(r"@(name)=(handler)"));
    }

    #[test]
    fn generates_dynamic_attributes_for_spread() {
        // A spread contributes an unknown set of keys.
        assert!(is_dynamic(r"(attrs)"));
    }

    #[test]
    fn generates_dynamic_attributes_for_if() {
        // Separate conditionals could each emit the same key.
        assert!(is_dynamic(r#"if cond { class="a" } else { class="b" }"#));
    }

    #[test]
    fn generates_dynamic_attributes_for_for_loop() {
        // A loop body may run repeatedly, duplicating its keys.
        assert!(is_dynamic(r#"for _ in items { class="a" }"#));
    }

    #[test]
    fn generates_dynamic_attributes_for_match() {
        // Match arms could emit overlapping keys with the surrounding list.
        assert!(is_dynamic(r#"match kind { _ => class="a", }"#));
    }
}
