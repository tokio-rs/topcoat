use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
    parse_quote,
};

use crate::ast::{
    ParseOption,
    view::{ViewWriter, WriteView},
};

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

impl WriteView for SignalDeclaration {
    fn write(&self, writer: &mut ViewWriter) {
        let ident = &self.ident;
        let expr = &self.expr;
        writer.let_binding(&parse_quote! { #ident }, expr);
        writer.let_binding(
            &parse_quote! { #ident },
            &parse_quote! { ::topcoat::runtime::Signal::new(&#ident) },
        );
        writer.write_expr(quote! { ::topcoat::runtime::SignalDeclaration::new(#ident) });
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
impl topcoat_pretty::PrettyPrint for SignalDeclaration {
    fn pretty_print(&self, _printer: &mut topcoat_pretty::Printer<'_>) {
        todo!();
    }
}
