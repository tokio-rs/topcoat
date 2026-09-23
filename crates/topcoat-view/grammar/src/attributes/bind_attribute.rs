use quote::quote;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_runtime};

use crate::{
    attributes::{
        AttributeKey,
        hir::{AttributeBuilder, LowerAttribute},
    },
    template::TemplateOrRuntimeExpr,
    view::hir::{ExprKind, LowerView, ViewBuilder},
};

/// A `:name=(expr)` or `:name=$(expr)` bind attribute, which keeps a DOM
/// attribute or property in sync with an expression in the browser.
///
/// The attribute renders with the expression's current value. Unless the
/// value is static, a `data-topcoat-bind:name` attribute holding the
/// expression's JavaScript form is rendered next to it.
pub struct BindAttribute {
    /// The leading `:`.
    pub colon: Token![:],
    /// The bound attribute name.
    pub key: AttributeKey,
    /// The `=` between name and value.
    pub eq: Token![=],
    /// The bound expression.
    pub value: TemplateOrRuntimeExpr,
}

impl LowerView for BindAttribute {
    fn lower(&self, builder: &mut ViewBuilder) {
        let key = &self.key;
        let value = &self.value;
        builder.expr(
            ExprKind::Attributes,
            quote! {
                #topcoat_runtime::BindAttribute::new(#key, #value)
            },
        );
    }
}

impl LowerAttribute for BindAttribute {
    fn lower(&self, builder: &mut AttributeBuilder) {
        let key = &self.key;
        let value = &self.value;
        builder.insert_block(
            2,
            quote! {
                {
                    let __key = ::core::convert::Into::<::std::string::String>::into(#key);
                    let __value = #value;
                    let __is_static = __value.is_static();
                    let (__evaluated, __js) = __value.into_evaluated_and_js();
                    __attrs.insert(__cx, __key.clone(), __evaluated);
                    let __binding_key = ::std::format!("data-topcoat-bind:{}", __key);
                    if __is_static {
                        __attrs.remove(&__binding_key);
                    } else {
                        __attrs.insert(__cx, __binding_key, __js);
                    }
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
impl topcoat_core_grammar::pretty::PrettyPrint for BindAttribute {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
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
