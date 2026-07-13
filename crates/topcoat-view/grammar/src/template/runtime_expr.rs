use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Token, parenthesized,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_runtime_macro;

use crate::view::{ExprKind, ViewWriter, WriteView};

/// A `$(`...`)` runtime expression, lowered through `runtime::expr!`.
#[derive(Debug, PartialEq)]
pub struct RuntimeExpr {
    pub dollar: Token![$],
    pub paren: syn::token::Paren,
    pub expr: syn::Expr,
}

impl WriteView for RuntimeExpr {
    fn write(&self, writer: &mut ViewWriter) {
        writer.write_expr(ExprKind::Node, self.to_token_stream());
    }
}

impl Parse for RuntimeExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            dollar: input.parse()?,
            paren: parenthesized!(content in input),
            expr: content.parse()?,
        })
    }
}

impl ParseOption for RuntimeExpr {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![$])
    }
}

impl ToTokens for RuntimeExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        quote! {
            #topcoat_runtime_macro::expr! { #expr }
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for RuntimeExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        "$".pretty_print(printer);
        "(".pretty_print(printer);
        self.expr.pretty_print(printer);
        ")".pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, quote};

    use super::*;

    #[test]
    fn tokens_wrap_expression() {
        let expr = syn::parse_str::<RuntimeExpr>("$(value + 1)").unwrap();
        assert_eq!(
            expr.to_token_stream().to_string(),
            quote! { #topcoat_runtime_macro::expr! { value + 1 } }.to_string(),
        );
    }
}
