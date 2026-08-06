use quote::quote;
use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::{
        AttributeKey, AttributeValue,
        hir::{AttributeBuilder, LowerAttribute},
    },
    view::hir::{ExprKind, LowerView, ViewBuilder},
};

/// A `name=value` attribute on an element, or an entry in an
/// [`Attributes`](super::Attributes) list.
///
/// The key may be a static [`HtmlIdent`](super::super::view::HtmlIdent) or a
/// parenthesized Rust expression; the value may be a string literal or a
/// parenthesized expression.
pub struct Attribute {
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: AttributeValue,
}

impl LowerView for Attribute {
    fn lower(&self, builder: &mut ViewBuilder) {
        match self.value {
            AttributeValue::LitStr(_) => {
                builder.write_str_unescaped(" ");
                self.key.lower(builder);
                builder.write_str_unescaped("=\"");
                self.value.lower(builder);
                builder.write_str_unescaped("\"");
            }
            AttributeValue::Expr(_) => {
                let key = &self.key;
                let value = &self.value;
                if matches!(key, AttributeKey::Expr(..)) {
                    builder.write_expr(ExprKind::Attribute, quote! { (#key, #value) });
                } else {
                    builder.write_expr(ExprKind::AttributeUnescaped, quote! { (#key, #value) });
                }
            }
        }
    }
}

impl LowerAttribute for Attribute {
    fn lower(&self, builder: &mut AttributeBuilder) {
        let key = &self.key;
        let value = &self.value;
        builder.insert(quote! { #key }, quote! { #value });
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Attribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident::peek_any) || input.peek(Paren)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Attribute {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Attribute {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Attribute>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_literal_key_and_value() {
        let attr = parse(r#"class="button""#);
        assert!(matches!(attr.key, AttributeKey::Ident(_)));
        assert!(matches!(attr.value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn parses_expression_value() {
        let attr = parse(r"href=(url)");
        assert!(matches!(attr.value, AttributeValue::Expr(_)));
    }

    #[test]
    fn parses_expression_key_and_value() {
        let attr = parse(r"(name)=(value)");
        assert!(matches!(attr.key, AttributeKey::Expr(_)));
        assert!(matches!(attr.value, AttributeValue::Expr(_)));
    }

    #[test]
    fn parses_html_ident_key() {
        let attr = parse(r#"data-post-id="42""#);
        assert_eq!(attr.key.to_string(), "data-post-id");
    }

    #[test]
    fn requires_equals_sign() {
        assert!(parse_err("class").contains("expected `=`"));
    }
}
