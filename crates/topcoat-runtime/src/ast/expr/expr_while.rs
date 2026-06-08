use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprWhile;

use crate::ast::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_while(
        expr: &ExprWhile,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut cond = TokenStream::new();
        js.push_str("while (");
        Self::dispatch(&expr.cond, &mut cond, js, names)?;
        js.push_str(".valueOf()) ");
        quote! { while ::topcoat::runtime::Surrogate::into_real(#cond) }.to_tokens(rust);

        Self::block(&expr.body, rust, js, names)?;
        Ok(())
    }
}
