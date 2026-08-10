use quote::quote;
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
    parse_quote,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_runtime};

use crate::view::hir::{ExprKind, LowerView, ViewBuilder};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(signal);
}

pub struct SignalDeclaration {
    pub signal_kw: kw::signal,
    pub ident: Ident,
    pub eq_token: Token![=],
    pub expr: Expr,
    pub semi_token: Token![;],
}

impl LowerView for SignalDeclaration {
    fn lower(&self, builder: &mut ViewBuilder) {
        let ident = &self.ident;
        let expr = &self.expr;
        builder.local_binding(&parse_quote! { #ident }, expr);
        builder.local_binding(
            &parse_quote! { #ident },
            &parse_quote! { &#topcoat_runtime::Signal::new(#ident) },
        );
        builder.expr(
            ExprKind::Node,
            quote! { #topcoat_runtime::SignalDeclaration::new(#ident) },
        );
    }
}

impl Parse for SignalDeclaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            signal_kw: input.parse()?,
            ident: input.parse()?,
            eq_token: input.parse()?,
            expr: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for SignalDeclaration {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::signal)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for SignalDeclaration {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        printer.move_cursor(self.signal_kw.span().start());
        "signal".pretty_print(printer);
        printer.move_cursor(self.signal_kw.span().end());
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}
