use quote::quote;
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
    parse_quote,
};

use topcoat_core::ast::ParseOption;

use crate::ast::view::{ExprKind, ViewWriter, WriteView};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(client);
}

pub struct ClientHandle {
    pub client_kw: kw::client,
    pub let_token: Token![let],
    pub ident: Ident,
    pub eq_token: Token![=],
    pub expr: Expr,
    pub semi_token: Token![;],
}

impl WriteView for ClientHandle {
    fn write(&self, writer: &mut ViewWriter) {
        let ident = &self.ident;
        let expr = &self.expr;
        writer.let_binding(&parse_quote! { #ident }, expr);
        writer.let_binding(
            &parse_quote! { #ident },
            &parse_quote! { &::topcoat::runtime::ClientHandle::new(#ident) },
        );
        writer.write_expr(
            ExprKind::Node,
            quote! { ::topcoat::runtime::ClientHandleDeclaration::new(#ident) },
        );
    }
}

impl Parse for ClientHandle {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            client_kw: input.parse()?,
            let_token: input.parse()?,
            ident: input.parse()?,
            eq_token: input.parse()?,
            expr: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for ClientHandle {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::client)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ClientHandle {
    fn pretty_print(&self, _printer: &mut topcoat_pretty::Printer<'_>) {
        todo!();
    }
}
