use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprAwait;

use crate::ast::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_await(
        expr: &ExprAwait,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        *js += "(await ";
        Self::dispatch(&expr.base, rust, js, names)?;
        quote! { .await }.to_tokens(rust);
        *js += ")";
        Ok(())
    }
}
