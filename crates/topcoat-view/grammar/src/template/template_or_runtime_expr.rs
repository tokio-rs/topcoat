use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::{ParseOption, paths::topcoat_runtime};

use crate::template::{RuntimeExpr, TemplateExpr};

/// The value of a bind attribute or event handler: either a plain `(expr)` or
/// a `$(expr)` runtime expression.
///
/// Both expand to a runtime `Expr` value. A plain expression is converted
/// into one with `From`.
#[derive(Debug, PartialEq)]
pub enum TemplateOrRuntimeExpr {
    /// A plain `(expr)`.
    Template(TemplateExpr),
    /// A `$(expr)` runtime expression.
    Runtime(RuntimeExpr),
}

impl Parse for TemplateOrRuntimeExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if RuntimeExpr::peek(input) {
            Ok(Self::Runtime(input.parse()?))
        } else if TemplateExpr::peek(input) {
            Ok(Self::Template(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ParseOption for TemplateOrRuntimeExpr {
    fn peek(input: ParseStream) -> bool {
        RuntimeExpr::peek(input) || TemplateExpr::peek(input)
    }
}

impl ToTokens for TemplateOrRuntimeExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Template(inner) => {
                quote! { #topcoat_runtime::Expr::from(#inner) }.to_tokens(tokens);
            }
            Self::Runtime(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TemplateOrRuntimeExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Template(inner) => inner.pretty_print(printer),
            Self::Runtime(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, quote};
    use topcoat_core_grammar::paths::topcoat_runtime_macro;

    use super::*;

    #[test]
    fn parses_template_expr() {
        assert!(matches!(
            syn::parse_str::<TemplateOrRuntimeExpr>("(value)").unwrap(),
            TemplateOrRuntimeExpr::Template(_),
        ));
    }

    #[test]
    fn parses_runtime_expr() {
        assert!(matches!(
            syn::parse_str::<TemplateOrRuntimeExpr>("$(value)").unwrap(),
            TemplateOrRuntimeExpr::Runtime(_),
        ));
    }

    #[test]
    fn template_expr_tokens_are_raw_expression() {
        let expr = syn::parse_str::<TemplateOrRuntimeExpr>("(value + 1)").unwrap();
        assert_eq!(
            expr.to_token_stream().to_string(),
            quote! { #topcoat_runtime::Expr::from(value + 1) }.to_string(),
        );
    }

    #[test]
    fn runtime_expr_tokens_wrap_expression() {
        let expr = syn::parse_str::<TemplateOrRuntimeExpr>("$(value + 1)").unwrap();
        assert_eq!(
            expr.to_token_stream().to_string(),
            quote! { #topcoat_runtime_macro::expr! { value + 1 } }.to_string(),
        );
    }
}
