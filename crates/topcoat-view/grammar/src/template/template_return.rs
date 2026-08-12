use quote::quote;
use syn::{
    ExprReturn, Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::view::hir::{LowerView, ViewBuilder};

/// A `return expr;` statement, ending the component's render.
///
/// Returning `Err` propagates the error out of the component; a component
/// that returns `Ok` stops rendering and keeps its last output.
pub struct TemplateReturn {
    pub expr_return: ExprReturn,
    pub semi_token: Token![;],
}

impl LowerView for TemplateReturn {
    fn lower(&self, builder: &mut ViewBuilder) {
        let expr_return = &self.expr_return;
        builder.statement(quote! { #expr_return; });
    }
}

impl Parse for TemplateReturn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_return: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateReturn {
    fn peek(input: ParseStream) -> bool {
        // `return=` is an attribute named `return`, not a statement.
        input.peek(Token![return]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TemplateReturn {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use quote::ToTokens;

        self.expr_return
            .to_token_stream()
            .to_string()
            .pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}
