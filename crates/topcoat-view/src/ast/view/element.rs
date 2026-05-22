use std::sync::atomic::Ordering;

use proc_macro2::Span;
use quote::quote;
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};

use crate::ast::{
    ParseOption,
    view::{Attributes, ClosingTag, ElementName, Node, Nodes, OpeningTag, ViewWriter, WriteView},
};

/// An HTML element. `Void` covers the HTML void elements (`<br>`, `<img>`, …)
/// which take no closing tag and no children.
// Optimize for the common case (normal elements).
#[allow(clippy::large_enum_variant)]
pub enum Element {
    Normal {
        opening_tag: OpeningTag,
        children: Nodes,
        closing_tag: ClosingTag,
    },
    Void {
        tag: OpeningTag,
    },
}

impl Element {
    /// The element's tag name.
    pub fn name(&self) -> &ElementName {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.name,
            Self::Void { tag } => &tag.name,
        }
    }

    /// The attributes on the opening tag.
    pub fn attributes(&self) -> &Attributes {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.attributes,
            Self::Void { tag } => &tag.attributes,
        }
    }

    /// The element's children. Always empty for void elements.
    pub fn children(&self) -> &[Node] {
        match self {
            Self::Normal { children, .. } => children,
            Self::Void { .. } => &[],
        }
    }
}

impl WriteView for Element {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Normal {
                opening_tag,
                children,
                ..
            } => {
                // For expression attribute names, we only want to evaluate the expression once and
                // then store it in a variable.
                let name_expr = opening_tag.name.expr();
                static AUTO_INCREMENT: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let increment = AUTO_INCREMENT.fetch_add(1, Ordering::Relaxed);
                let name_ident = name_expr.map(|_| {
                    Ident::new(&format!("__element_name_{}", increment), Span::call_site())
                });

                writer.write_str_unescaped("<");
                match (name_ident.as_ref(), name_expr) {
                    (Some(ident), Some(expr)) => {
                        writer.let_binding(&syn::parse_quote!(#ident), &syn::parse_quote!(&#expr));
                        writer.write_expr(quote! { #ident });
                    }
                    _ => opening_tag.name.write(writer),
                }
                opening_tag.attributes.write(writer);
                writer.write_str_unescaped(">");

                for child in children {
                    child.write(writer);
                }

                writer.write_str_unescaped("</");
                match name_ident {
                    Some(ident) => writer.write_expr(quote! { #ident }),
                    _ => opening_tag.name.write(writer),
                }
                writer.write_str_unescaped(">");
            }
            Self::Void { tag } => {
                writer.write_str_unescaped("<");
                tag.name.write(writer);
                tag.attributes.write(writer);
                writer.write_str_unescaped(">");
            }
        }
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let opening_tag: OpeningTag = input.parse()?;

        if opening_tag.name.is_void_element() {
            return Ok(Self::Void { tag: opening_tag });
        }

        let children: Nodes = input.parse()?;

        if input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                format!("missing closing tag for opening tag `{}`", opening_tag.name),
            ));
        }
        let closing_tag: ClosingTag = input.parse()?;
        if closing_tag.name != opening_tag.name {
            return Err(syn::Error::new(
                closing_tag.name.span(),
                format!(
                    "closing tag `{}` does not match opening tag `{}`",
                    closing_tag.name, opening_tag.name
                ),
            ));
        }
        Ok(Self::Normal {
            opening_tag,
            children,
            closing_tag,
        })
    }
}

impl ParseOption for Element {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![<])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Element {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        printer.scan_begin(topcoat_pretty::BreakMode::Consistent);
        match self {
            Self::Normal {
                opening_tag,
                children,
                closing_tag,
            } => {
                opening_tag.pretty_print(printer);
                printer.scan_indent(1);
                printer.scan_break();
                printer.scan_trivia(false, true);
                children.pretty_print(printer);
                printer.scan_same_line_trivia();
                printer.scan_trivia(true, false);
                printer.scan_indent(-1);
                printer.scan_break();
                closing_tag.pretty_print(printer);
            }
            Self::Void { tag } => tag.pretty_print(printer),
        }
        printer.scan_end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Element {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Element>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_normal_element() {
        let element = parse("<div></div>");
        assert!(matches!(element, Element::Normal { .. }));
        assert_eq!(element.name().string_name().as_deref(), Some("div"));
        assert!(element.attributes().is_empty());
        assert!(element.children().is_empty());
    }

    #[test]
    fn parses_void_element_without_closing_tag() {
        let element = parse("<br>");
        assert!(matches!(element, Element::Void { .. }));
        assert!(element.children().is_empty());
    }

    #[test]
    fn parses_nested_children() {
        let element = parse(r#"<div><p>"hi"</p></div>"#);
        assert_eq!(element.children().len(), 1);
    }

    #[test]
    fn missing_closing_tag_is_rejected() {
        assert!(parse_err("<div>").contains("missing closing tag"));
    }

    #[test]
    fn mismatched_closing_tag_is_rejected() {
        assert!(parse_err("<div></span>").contains("does not match"));
    }
}
