use quote::quote;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{AttributeKey, AttributeWriter, WriteAttribute},
    template::TemplateOrRuntimeExpr,
    view::{ExprKind, ViewWriter, WriteView},
};

/// A `:name=(expr)` or `:name=$(expr)` attribute: a one-way binding to a DOM
/// attribute or property.
pub struct BindAttribute {
    pub colon: Token![:],
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: TemplateOrRuntimeExpr,
}

impl WriteView for BindAttribute {
    fn write(&self, writer: &mut ViewWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.write_expr(
            ExprKind::Attributes,
            quote! {
                ::topcoat::runtime::BindAttribute::new(#key, #value)
            },
        );
    }
}

impl WriteAttribute for BindAttribute {
    fn write(&self, writer: &mut AttributeWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.insert_block(
            2,
            quote! {
                {
                    let __key = ::core::convert::Into::<::std::string::String>::into(#key);
                    let (__evaluated, __js) = #value.into_evaluated_and_js();
                    __attrs.insert(__key.clone(), __evaluated);
                    __attrs.insert(::std::format!("data-topcoat-bind:{}", __key), __js);
                }
            },
        );
    }
}

impl Parse for BindAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            colon: input.parse()?,
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for BindAttribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![:])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for BindAttribute {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.colon.pretty_print(printer);
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> BindAttribute {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<BindAttribute>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_template_expr_value() {
        let attr = parse(":value=(v)");
        assert!(matches!(attr.key, AttributeKey::Ident(_)));
        assert!(matches!(attr.value, TemplateOrRuntimeExpr::Template(_)));
    }

    #[test]
    fn parses_runtime_expr_value() {
        let attr = parse(":value=$(v)");
        assert!(matches!(attr.value, TemplateOrRuntimeExpr::Runtime(_)));
    }

    #[test]
    fn parses_expression_key() {
        let attr = parse(":(name)=(v)");
        assert!(matches!(attr.key, AttributeKey::Expr(_)));
    }

    #[test]
    fn parses_html_ident_key() {
        let attr = parse(":data-foo=(v)");
        let AttributeKey::Ident(ident) = &attr.key else {
            panic!("expected ident key");
        };
        assert_eq!(ident.to_string(), "data-foo");
    }

    #[test]
    fn rejects_literal_value() {
        // Bindings must carry a Rust expression, not a string literal.
        assert!(parse_err(r#":value="v""#).contains("expected"));
    }
}
